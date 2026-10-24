# Daily-use retained-image fixtures

`make-images.py` uses only Python's standard library and the existing synthetic
Harbor DOCX and Quay JPEG. It deterministically writes:

- `jpeg-figure.docx`: existing Harbor text/layout with its image replaced by exact
  `crates/brn-intake/tests/fixtures/quay-map.jpg` bytes. Raster dimensions differ
  from the inherited Office display extents.
- `invalid-image.docx`: the same package with an invalid image payload.
- `four-pixels.pdf`: hand-authored one-page PDF containing text and one 2×2 RGB
  image (red, green, blue, white), with no external resources.

The existing Harbor PNG DOCX and two-image Quay PPTX are reused byte-for-byte.
Tests independently check asset counts, native image hashes/bytes, source hashes,
managed link destinations and unchanged non-destination Markdown. Restricted
helper tests and checked host conversion are distinct from direct SDK extraction.
These fixtures do not establish general Office/PDF fidelity or native UI quality.
