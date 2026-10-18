//! Linux x86_64 pipe-only helper policy, installed after the dynamic loader and
//! before document input or worker creation. Other Linux architectures refuse.
//!
//! Kernel contract: https://docs.kernel.org/userspace-api/seccomp_filter.html
//! glibc pthread/clone fallback contract:
//! https://sourceware.org/git/?p=glibc.git;a=blob;f=sysdeps/unix/sysv/linux/clone-internal.c
//! https://sourceware.org/git/?p=glibc.git;a=blob;f=nptl/pthread_create.c
//! No pathname operations, sockets, executable mappings, exec, process-group
//! changes or cross-process interfaces are admitted. Existing descriptors above
//! stderr are closed before filtering. Children inherit the filter; the host
//! continues owning their process group, wall deadline and bounded protocol pipes.

use brn_intake::MAX_OUTPUT_BYTES;

const DENY: u32 = libc::SECCOMP_RET_ERRNO | libc::EPERM as u32;
const ALLOW: u32 = libc::SECCOMP_RET_ALLOW;
const LOAD: u16 = (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16;
const EQUAL: u16 = (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16;
const RETURN: u16 = (libc::BPF_RET | libc::BPF_K) as u16;

fn instruction(code: u16, jt: u8, jf: u8, k: u32) -> libc::sock_filter {
    libc::sock_filter { code, jt, jf, k }
}

// Each rule ends in a return and otherwise leaves the syscall number in A.
fn allow(filter: &mut Vec<libc::sock_filter>, syscall: libc::c_long) {
    filter.push(instruction(EQUAL, 0, 1, syscall as u32));
    filter.push(instruction(RETURN, 0, 0, ALLOW));
}

fn argument_rule(
    filter: &mut Vec<libc::sock_filter>,
    syscall: libc::c_long,
    checks: &[(u32, u32, u32)],
    action: u32,
) {
    // seccomp_data.args begins at byte 16; each argument is 64 bits. Each
    // check compares a masked 32-bit word. Callers check high words as needed.
    let length = checks.len() * 4 + 1;
    filter.push(instruction(EQUAL, 0, length as u8, syscall as u32));
    for &(offset, mask, value) in checks {
        filter.push(instruction(LOAD, 0, 0, offset));
        filter.push(instruction(
            (libc::BPF_ALU | libc::BPF_AND | libc::BPF_K) as u16,
            0,
            0,
            mask,
        ));
        filter.push(instruction(EQUAL, 1, 0, value));
        filter.push(instruction(RETURN, 0, 0, DENY));
    }
    filter.push(instruction(RETURN, 0, 0, action));
}

fn policy(process: u32, process_tree_witness: bool) -> Vec<libc::sock_filter> {
    let mut filter = vec![
        instruction(LOAD, 0, 0, 4),            // seccomp_data.arch
        instruction(EQUAL, 1, 0, 0xc000_003e), // AUDIT_ARCH_X86_64
        instruction(RETURN, 0, 0, libc::SECCOMP_RET_KILL_PROCESS),
        instruction(LOAD, 0, 0, 0), // seccomp_data.nr
        // x32 shares AUDIT_ARCH_X86_64 but sets this syscall-number bit.
        instruction(
            (libc::BPF_JMP | libc::BPF_JSET | libc::BPF_K) as u16,
            0,
            1,
            0x4000_0000,
        ),
        instruction(RETURN, 0, 0, libc::SECCOMP_RET_KILL_PROCESS),
    ];
    // Pipe I/O and descriptor metadata, allocator, signals, Rust/glibc thread
    // coordination, monotonic deadlines, random seeds and process exit.
    for syscall in [
        libc::SYS_read,
        libc::SYS_write,
        libc::SYS_readv,
        libc::SYS_writev,
        libc::SYS_close,
        libc::SYS_fstat,
        libc::SYS_lseek,
        libc::SYS_brk,
        libc::SYS_munmap,
        libc::SYS_mremap,
        libc::SYS_madvise,
        libc::SYS_rt_sigaction,
        libc::SYS_rt_sigprocmask,
        libc::SYS_rt_sigreturn,
        libc::SYS_sigaltstack,
        libc::SYS_futex,
        libc::SYS_sched_yield,
        libc::SYS_sched_getaffinity,
        libc::SYS_getpid,
        libc::SYS_gettid,
        libc::SYS_getrandom,
        libc::SYS_clock_gettime,
        libc::SYS_clock_nanosleep,
        libc::SYS_nanosleep,
        libc::SYS_set_robust_list,
        libc::SYS_rseq,
        libc::SYS_exit,
        libc::SYS_exit_group,
        // Tokio's in-memory wakeup/reactor descriptors confer no external I/O.
        libc::SYS_epoll_create1,
        libc::SYS_epoll_ctl,
        libc::SYS_epoll_wait,
        libc::SYS_epoll_pwait,
        libc::SYS_eventfd2,
    ] {
        allow(&mut filter, syscall);
    }
    // No file-backed or executable mappings. mmap's protection is argument 2,
    // flags argument 3; mmap ignores fd when MAP_ANONYMOUS is set.
    argument_rule(
        &mut filter,
        libc::SYS_mmap,
        &[
            (32, libc::PROT_EXEC as u32, 0),
            (36, u32::MAX, 0),
            (40, libc::MAP_ANONYMOUS as u32, libc::MAP_ANONYMOUS as u32),
            (44, u32::MAX, 0),
            // Anonymous mappings need no backing-file offset.
            (56, u32::MAX, 0),
            (60, u32::MAX, 0),
        ],
        ALLOW,
    );
    argument_rule(
        &mut filter,
        libc::SYS_mprotect,
        &[(32, libc::PROT_EXEC as u32, 0), (36, u32::MAX, 0)],
        ALLOW,
    );
    // BPF cannot dereference clone3's flags pointer. Explicit ENOSYS invokes
    // glibc's documented legacy-clone fallback; it never reports clone success.
    filter.push(instruction(EQUAL, 0, 1, libc::SYS_clone3 as u32));
    filter.push(instruction(
        RETURN,
        0,
        0,
        libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32,
    ));
    // Exact glibc pthread flags. All workers share CPU/AS process limits. Deny
    // namespaces, CLONE_PARENT and unknown flags, including high-word bits.
    let thread_flags = (libc::CLONE_VM
        | libc::CLONE_FS
        | libc::CLONE_FILES
        | libc::CLONE_SYSVSEM
        | libc::CLONE_SIGHAND
        | libc::CLONE_THREAD
        | libc::CLONE_SETTLS
        | libc::CLONE_PARENT_SETTID
        | libc::CLONE_CHILD_CLEARTID) as u32;
    let clone_flags = if process_tree_witness {
        // This exact argv branch only creates the two trusted witness children,
        // prints their PIDs and sleeps. It never reads or parses document input.
        allow(&mut filter, libc::SYS_fork);
        (libc::CLONE_CHILD_SETTID | libc::CLONE_CHILD_CLEARTID | libc::SIGCHLD) as u32
    } else {
        thread_flags
    };
    argument_rule(
        &mut filter,
        libc::SYS_clone,
        &[(16, u32::MAX, clone_flags), (20, u32::MAX, 0)],
        ALLOW,
    );
    // glibc abort can signal this process, never the host or a neighbor. Forked
    // witness descendants can still exit normally; they need no signal syscall.
    argument_rule(
        &mut filter,
        libc::SYS_tgkill,
        &[(16, u32::MAX, process), (20, u32::MAX, 0)],
        ALLOW,
    );
    filter.push(instruction(RETURN, 0, 0, DENY));
    filter
}

pub(super) fn restrict(process_tree_witness: bool) -> Result<(), String> {
    // The linked loader has finished. Drop all ambient descriptors before any
    // untrusted input. Unsupported close_range is a refusal, never a fallback.
    if unsafe { libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 0u32) } != 0 {
        return Err("helper descriptor restriction activation failed".into());
    }
    for (resource, limit) in [
        (libc::RLIMIT_CPU, 10),
        (libc::RLIMIT_NOFILE, 64),
        (libc::RLIMIT_FSIZE, MAX_OUTPUT_BYTES as u64),
        (libc::RLIMIT_AS, 1024 * 1024 * 1024),
        (libc::RLIMIT_CORE, 0),
    ] {
        let limits = libc::rlimit {
            rlim_cur: limit,
            rlim_max: limit,
        };
        if unsafe { libc::setrlimit(resource, &limits) } != 0 {
            return Err("helper resource restriction activation failed".into());
        }
    }
    if unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0 {
        return Err("helper privilege restriction activation failed".into());
    }
    let filter = policy(std::process::id(), process_tree_witness);
    let program = libc::sock_fprog {
        len: filter.len() as u16,
        filter: filter.as_ptr().cast_mut(),
    };
    // Synchronize every existing thread and make future threads inherit the
    // same policy. A positive unsynchronized TID is also failure, not success.
    if unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            libc::SECCOMP_SET_MODE_FILTER,
            libc::SECCOMP_FILTER_FLAG_TSYNC,
            &program,
        )
    } != 0
    {
        return Err("helper syscall restriction activation failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn pipe_only_restrictions_in_child() {
        let path = std::env::temp_dir().join(format!(
            "brn-linux-restriction-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"synthetic forbidden content").unwrap();
        let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(std::fs::read(&path).is_ok());
        assert!(std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).is_ok());
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .env_clear()
            .env("BRN_RESTRICTION_TEST_FILE", &path)
            .env("BRN_RESTRICTION_TEST_PORT", port.to_string())
            .args([
                "--ignored",
                "--exact",
                "helper_runtime::linux::tests::restricted_child",
                "--test-threads=1",
                "--nocapture",
            ]);
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "restriction child failed: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("restriction witness passed"));
        // An x32 syscall has the same audit architecture as x86_64; it must be
        // rejected before any native syscall allow rule can match.
        use std::os::unix::process::ExitStatusExt;
        let x32 = command
            .env("BRN_RESTRICTION_TEST_X32", "1")
            .output()
            .unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(x32.status.signal(), Some(libc::SIGSYS));
    }

    #[test]
    #[ignore = "only run inside pipe_only_restrictions_in_child"]
    fn restricted_child() {
        use std::os::fd::IntoRawFd;
        let path = std::env::var_os("BRN_RESTRICTION_TEST_FILE").unwrap();
        let port: u16 = std::env::var("BRN_RESTRICTION_TEST_PORT")
            .unwrap()
            .parse()
            .unwrap();
        let x32 = std::env::var_os("BRN_RESTRICTION_TEST_X32").is_some();
        // Deliberately leave one readable descriptor above stderr. Restriction
        // activation must remove it, not merely deny opening its pathname.
        let inherited = std::fs::File::open(&path).unwrap().into_raw_fd();
        assert!(inherited > 2);
        super::restrict(false).unwrap();
        if x32 {
            unsafe { libc::syscall(0x4000_0000 | libc::SYS_getpid) };
            panic!("x32 syscall survived architecture rejection");
        }
        let mut byte = 0u8;
        assert_eq!(
            unsafe { libc::read(inherited, (&mut byte as *mut u8).cast(), 1) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EBADF)
        );
        let denied = move || {
            assert_eq!(
                std::fs::read(&path).unwrap_err().raw_os_error(),
                Some(libc::EPERM)
            );
            assert_eq!(
                std::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port))
                    .unwrap_err()
                    .raw_os_error(),
                Some(libc::EPERM)
            );
        };
        denied();
        std::thread::spawn(denied).join().unwrap();
        assert_eq!(unsafe { libc::fork() }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EPERM)
        );
        assert_eq!(unsafe { libc::setsid() }, -1);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EPERM)
        );
        let executable = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_EXEC,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_eq!(executable, libc::MAP_FAILED);
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::EPERM)
        );
        println!("restriction witness passed");
    }
}
