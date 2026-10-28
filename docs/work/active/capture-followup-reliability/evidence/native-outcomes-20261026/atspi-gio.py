#!/usr/bin/python3
"""One-off native evidence probe using existing Gio, without Python Atspi bindings."""
import argparse, json, sys, time
from gi.repository import Gio, GLib
p=argparse.ArgumentParser();p.add_argument('mode',choices=['dump','action','point','value']);p.add_argument('--name');p.add_argument('--role');a=p.parse_args()
session=Gio.bus_get_sync(Gio.BusType.SESSION,None)
def call(conn,bus,path,interface,method,args=None):
    return conn.call_sync(bus,path,interface,method,args,None,Gio.DBusCallFlags.NONE,1000,None).unpack()
address=call(session,'org.a11y.Bus','/org/a11y/bus','org.a11y.Bus','GetAddress')[0]
bus=Gio.DBusConnection.new_for_address_sync(address,Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT|Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION,None,None)
root=('org.a11y.atspi.Registry','/org/a11y/atspi/accessible/root')
queue=[(root,0)];seen=set();nodes=[];deadline=time.monotonic()+15
while queue and len(nodes)<2000 and time.monotonic()<deadline:
    (destination,path),depth=queue.pop(0)
    if (destination,path) in seen: continue
    if depth>30:
        nodes.append({'error':'Traversal depth bound reached'});continue
    seen.add((destination,path));node={'bus':destination,'path':path,'depth':depth}
    try:
        node['name']=call(bus,destination,path,'org.freedesktop.DBus.Properties','Get',GLib.Variant('(ss)',('org.a11y.atspi.Accessible','Name')))[0]
        node['role']=call(bus,destination,path,'org.a11y.atspi.Accessible','GetRole')[0]
        node['state_words']=call(bus,destination,path,'org.a11y.atspi.Accessible','GetState')[0]
        bits=sum(word<<(32*i) for i,word in enumerate(node['state_words']))
        node['states']={name:bool(bits&(1<<bit)) for name,bit in [('enabled',8),('focusable',11),('focused',12),('sensitive',24),('showing',25),('visible',30)]}
        interfaces=call(bus,destination,path,'org.a11y.atspi.Accessible','GetInterfaces')[0]
        if 'org.a11y.atspi.Action' in interfaces:
            node['actions']=call(bus,destination,path,'org.a11y.atspi.Action','GetActions')[0]
        children=call(bus,destination,path,'org.a11y.atspi.Accessible','GetChildren')[0]
        queue.extend(((child[0] or destination,child[1]),depth+1) for child in children if child[1]!='/org/a11y/atspi/null')
    except (GLib.Error, IndexError, TypeError, ValueError) as error: node['error']=str(error)
    nodes.append(node)
complete=not queue and not any('error' in node for node in nodes)
if a.mode=='dump':
    print(json.dumps({'complete':complete,'nodes':nodes},indent=2));sys.exit(0 if complete else 2)
if not a.name: p.error('action requires --name EXACT')
if not complete: sys.exit('Incomplete tree; refusing action')
candidates=[n for n in nodes if n.get('name')==a.name and (not a.role or n.get('role')==a.role) and (a.mode in ('point','value') or n.get('actions'))]
if len(candidates)!=1: sys.exit(f'Expected one actionable exact-name match; found {len(candidates)}')
node=candidates[0]
if not all(node.get('states',{}).get(key) for key in ('enabled','sensitive','showing','visible')):
    sys.exit('Target is not currently exposed and enabled; refusing action')
if a.mode=='value':
    print(call(bus,node['bus'],node['path'],'org.a11y.atspi.Text','GetText',GLib.Variant('(ii)',(0,-1)))[0]);sys.exit(0)
if a.mode=='point':
    rect=call(bus,node['bus'],node['path'],'org.a11y.atspi.Component','GetExtents',GLib.Variant('(u)',(1,)))
    if len(rect)==1: rect=rect[0]
    if len(rect)!=4 or rect[2]<=0 or rect[3]<=0: sys.exit('Invalid target bounds')
    print(int(rect[0]+rect[2]/2),int(rect[1]+rect[3]/2));sys.exit(0)
result=call(bus,node['bus'],node['path'],'org.a11y.atspi.Action','DoAction',GLib.Variant('(i)',(0,)))[0]
print(json.dumps({'target':node,'action_accepted':result},indent=2));sys.exit(0 if result else 3)
