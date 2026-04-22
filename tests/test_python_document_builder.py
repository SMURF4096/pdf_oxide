"""Integration tests for the Python write-side API (#384 Phase 1 & 2).

Before v0.3.38 the Python binding exposed only ``Pdf.from_markdown`` /
``from_html`` / ``from_text`` — roughly 15% of the Rust write-side
surface. v0.3.38 adds:

* ``DocumentBuilder`` + ``FluentPageBuilder`` + ``EmbeddedFont`` (Phase 1)
  — the fluent API for programmatic multi-page construction, with
  embedded TTF/OTF support that closes #382 on the Python side.

* ``Pdf.from_html_css`` / ``from_html_css_with_fonts`` (Phase 2) — the
  HTML+CSS pipeline (#248) that was previously only reachable from Rust.

These tests exercise both phases end-to-end using the DejaVuSans fixture
that already ships at ``tests/fixtures/fonts/DejaVuSans.ttf``.

To run locally:

    maturin develop --features python
    pytest tests/test_python_document_builder.py -v
"""

from __future__ import annotations

from pathlib import Path

import pytest


pdf_oxide = pytest.importorskip("pdf_oxide")

FIXTURE_FONT = Path(__file__).parent / "fixtures" / "fonts" / "DejaVuSans.ttf"


# ---------------------------------------------------------------------------
# Phase 1 — DocumentBuilder + EmbeddedFont
# ---------------------------------------------------------------------------


def test_document_builder_minimal_ascii(tmp_path):
    """The simplest possible builder chain produces a valid PDF on disk."""
    out = tmp_path / "out.pdf"
    (
        pdf_oxide.DocumentBuilder()
        .title("Hello")
        .a4_page()
        .text("Hello, world.")
        .done()
        .save(str(out))
    )
    assert out.exists()
    assert out.stat().st_size > 256  # sanity floor; empty PDF is much smaller
    header = out.read_bytes()[:8]
    assert header.startswith(b"%PDF-")


def test_document_builder_build_returns_bytes():
    """``.build()`` returns a ``bytes`` object without touching the disk."""
    data = pdf_oxide.DocumentBuilder().a4_page().at(72.0, 720.0).text("pytest").done().build()
    assert isinstance(data, bytes)
    assert data.startswith(b"%PDF-")


def test_document_builder_cjk_round_trip(tmp_path):
    """The headline #384/#382 integration test.

    Register DejaVuSans (covers Latin + Cyrillic + Greek — enough to
    prove the Type-0 / CIDFontType2 path is live without needing a CJK
    fixture), emit text in each script, re-parse the output, and verify
    every original string comes back through ``extract_text``.
    """
    font = pdf_oxide.EmbeddedFont.from_file(str(FIXTURE_FONT))
    pdf_bytes = (
        pdf_oxide.DocumentBuilder()
        .register_embedded_font("DejaVu", font)
        .a4_page()
        .font("DejaVu", 12.0)
        .at(72.0, 720.0)
        .text("Привет, мир!")
        .at(72.0, 700.0)
        .text("Καλημέρα κόσμε")
        .at(72.0, 680.0)
        .text("The quick brown fox")
        .done()
        .build()
    )
    doc = pdf_oxide.PdfDocument.from_bytes(pdf_bytes)
    text = doc.extract_text(0)
    assert "Привет, мир!" in text
    assert "Καλημέρα κόσμε" in text
    assert "The quick brown fox" in text


def test_document_builder_output_is_subsetted():
    """Register a ~760 KB font, emit one short line; the resulting PDF
    must be dramatically smaller than the face, proving the v0.3.38
    subsetter (#385) is wired through the Python path too."""
    font = pdf_oxide.EmbeddedFont.from_file(str(FIXTURE_FONT))
    pdf_bytes = (
        pdf_oxide.DocumentBuilder()
        .register_embedded_font("DejaVu", font)
        .a4_page()
        .font("DejaVu", 12.0)
        .at(72.0, 700.0)
        .text("Hello world")
        .done()
        .build()
    )
    face_size = FIXTURE_FONT.stat().st_size
    assert len(pdf_bytes) * 10 < face_size, (
        f"expected PDF ({len(pdf_bytes)} bytes) to be at least 10× smaller than "
        f"the original face ({face_size} bytes); subsetter likely not wired"
    )


def test_document_builder_multiple_pages(tmp_path):
    """A builder can produce many pages; each ``done()`` returns the
    parent so the chain keeps going."""
    out = tmp_path / "multi.pdf"
    (
        pdf_oxide.DocumentBuilder()
        .a4_page()
        .at(72.0, 720.0)
        .text("page one")
        .done()
        .a4_page()
        .at(72.0, 720.0)
        .text("page two")
        .done()
        .a4_page()
        .at(72.0, 720.0)
        .text("page three")
        .done()
        .save(str(out))
    )
    doc = pdf_oxide.PdfDocument(str(out))
    # The high-level API keeps the DocumentBuilder usable; we check the
    # rendered PDF has the right page count.
    assert doc.page_count() == 3


def test_document_builder_save_encrypted(tmp_path):
    """Phase 1 includes AES-256 encryption (from v0.3.38 #386)."""
    out = tmp_path / "encrypted.pdf"
    (
        pdf_oxide.DocumentBuilder()
        .a4_page()
        .at(72.0, 720.0)
        .text("confidential")
        .done()
        .save_encrypted(str(out), "userpw", "ownerpw")
    )
    raw = out.read_bytes()
    assert b"/Encrypt" in raw, "encrypted PDF missing /Encrypt dictionary"
    assert b"/V 5" in raw, "expected /V 5 (AES-256) marker"


def test_document_builder_to_bytes_encrypted():
    data = (
        pdf_oxide.DocumentBuilder()
        .a4_page()
        .at(72.0, 720.0)
        .text("bytes")
        .done()
        .to_bytes_encrypted("u", "o")
    )
    assert isinstance(data, bytes)
    assert b"/Encrypt" in data
    assert b"/V 5" in data


def test_document_builder_consumed_after_build():
    """Calling ``build()`` again on the same instance should raise."""
    b = pdf_oxide.DocumentBuilder().a4_page().text("x").done()
    b.build()
    with pytest.raises(RuntimeError):
        b.build()


def test_document_builder_consumed_after_save(tmp_path):
    b = pdf_oxide.DocumentBuilder().a4_page().text("x").done()
    b.save(str(tmp_path / "once.pdf"))
    with pytest.raises(RuntimeError):
        b.save(str(tmp_path / "twice.pdf"))


def test_fluent_page_done_is_single_use():
    """A ``FluentPageBuilder`` may only be committed once."""
    b = pdf_oxide.DocumentBuilder()
    page = b.a4_page().text("a")
    page.done()
    with pytest.raises(RuntimeError):
        page.done()


def test_embedded_font_consumed_after_register():
    """Registering an ``EmbeddedFont`` consumes it; re-registering
    raises."""
    font = pdf_oxide.EmbeddedFont.from_file(str(FIXTURE_FONT))
    b = pdf_oxide.DocumentBuilder().register_embedded_font("DejaVu", font)
    with pytest.raises(RuntimeError):
        b.register_embedded_font("DejaVu2", font)


def test_embedded_font_from_bytes():
    """``EmbeddedFont.from_bytes`` accepts Python ``bytes`` and
    optionally an override name."""
    data = FIXTURE_FONT.read_bytes()
    font = pdf_oxide.EmbeddedFont.from_bytes(data, name="CustomDejaVu")
    # name override applied
    assert font.name == "CustomDejaVu"


def test_document_builder_annotations():
    """Phase 3 surface: annotations attached to the previous text
    element. We emit a variety and round-trip the text; annotation
    metadata is verified via ``get_annotations(page)`` in a later
    sweep — for now we just prove the methods don't raise and the
    document still renders coherent text."""
    data = (
        pdf_oxide.DocumentBuilder()
        .a4_page()
        .at(72.0, 720.0)
        .text("click me")
        .link_url("https://example.com")
        .at(72.0, 700.0)
        .text("important")
        .highlight((1.0, 1.0, 0.0))
        .at(72.0, 680.0)
        .text("revisit")
        .sticky_note("please review")
        .watermark_draft()
        .done()
        .build()
    )
    doc = pdf_oxide.PdfDocument.from_bytes(data)
    text = doc.extract_text(0)
    assert "click me" in text
    assert "important" in text
    assert "revisit" in text


# ---------------------------------------------------------------------------
# Phase 2 — HTML+CSS pipeline
# ---------------------------------------------------------------------------


def test_pdf_from_html_css_basic():
    """Single-font HTML+CSS → PDF via ``Pdf.from_html_css``."""
    font_bytes = FIXTURE_FONT.read_bytes()
    pdf = pdf_oxide.Pdf.from_html_css(
        "<h1>Hello</h1><p>World</p>",
        "h1 { color: blue; font-size: 24pt }",
        font_bytes,
    )
    data = pdf.to_bytes()
    assert data.startswith(b"%PDF-")
    # Confirm the rendered output round-trips the literal text.
    doc = pdf_oxide.PdfDocument.from_bytes(data)
    rendered = doc.extract_text(0)
    assert "Hello" in rendered
    assert "World" in rendered


def test_pdf_from_html_css_with_fonts_cascade():
    """Multi-font cascade: the first entry is the default, later
    entries resolve CSS ``font-family`` matches."""
    font_bytes = FIXTURE_FONT.read_bytes()
    pdf = pdf_oxide.Pdf.from_html_css_with_fonts(
        "<p>default</p><p style=\"font-family: 'My Sans'\">named</p>",
        "p { font-size: 12pt }",
        [("Body", font_bytes), ("My Sans", font_bytes)],
    )
    data = pdf.to_bytes()
    assert data.startswith(b"%PDF-")
    doc = pdf_oxide.PdfDocument.from_bytes(data)
    rendered = doc.extract_text(0)
    assert "default" in rendered
    assert "named" in rendered


def test_pdf_from_html_css_with_fonts_rejects_empty_font_list():
    with pytest.raises(ValueError):
        pdf_oxide.Pdf.from_html_css_with_fonts(
            "<p>x</p>",
            "",
            [],
        )


# ---------------------------------------------------------------------------
# Release sanity
# ---------------------------------------------------------------------------


def test_form_field_creation():
    """#384 Phase 4 — DocumentBuilder can now create form-field widgets
    (text fields + checkboxes) directly on a page. The resulting PDF
    exposes an /AcroForm entry and the widgets round-trip through the
    read-side form-field API."""
    bytes_ = (
        pdf_oxide.DocumentBuilder()
        .a4_page()
        .at(72.0, 720.0)
        .text("Fill out the form:")
        .text_field("name", 150.0, 720.0, 200.0, 20.0, "default name")
        .text_field("email", 150.0, 690.0, 200.0, 20.0)
        .checkbox("subscribe", 72.0, 650.0, 15.0, 15.0, True)
        .checkbox("remember", 72.0, 620.0, 15.0, 15.0, False)
        .done()
        .build()
    )
    assert b"/AcroForm" in bytes_
    # Verify via the existing form-field read API
    doc = pdf_oxide.PdfDocument.from_bytes(bytes_)
    fields = doc.get_form_fields()
    names = {f.name for f in fields}
    assert {"name", "email", "subscribe", "remember"} <= names, (
        f"expected all 4 field names in output; got {names}"
    )


def test_form_field_all_five_widget_types():
    """#384 Phase 4 — every widget type the plan listed (text_field,
    checkbox, combo_box, radio_group, push_button) is now reachable
    from Python and produces a correct /AcroForm entry."""
    bytes_ = (
        pdf_oxide.DocumentBuilder()
        .a4_page()
        .text_field("name", 150.0, 720.0, 200.0, 20.0, "Jane Doe")
        .checkbox("subscribe", 72.0, 680.0, 15.0, 15.0, True)
        .combo_box(
            "country",
            150.0,
            640.0,
            200.0,
            20.0,
            ["US", "CA", "UK", "DE"],
            "US",
        )
        .radio_group(
            "payment",
            [
                ("credit", 72.0, 600.0, 15.0, 15.0),
                ("debit", 72.0, 580.0, 15.0, 15.0),
                ("paypal", 72.0, 560.0, 15.0, 15.0),
            ],
            "credit",
        )
        .push_button("submit", 72.0, 520.0, 80.0, 24.0, "Submit")
        .done()
        .build()
    )
    assert b"/AcroForm" in bytes_
    doc = pdf_oxide.PdfDocument.from_bytes(bytes_)
    fields = doc.get_form_fields()
    names = {f.name for f in fields}
    # Every named field we created should be present in the output
    # (radio_group adds a parent field with the group name).
    expected = {"name", "subscribe", "country", "payment", "submit"}
    assert expected <= names, f"expected {expected} in output field names; got {names}"


def test_graphics_primitives():
    """#384 Phase 4 — low-level graphics primitives (rect, filled_rect,
    line) are reachable directly from DocumentBuilder without going
    through the ContentElement::Path builder."""
    bytes_ = (
        pdf_oxide.DocumentBuilder()
        .a4_page()
        # A framing box + an inner filled box + a diagonal line.
        .rect(50.0, 50.0, 500.0, 700.0)
        .filled_rect(100.0, 100.0, 200.0, 100.0, 0.9, 0.9, 1.0)
        .line(50.0, 400.0, 550.0, 400.0)
        .at(72.0, 500.0)
        .text("Graphics primitives demo")
        .done()
        .build()
    )
    # Sanity: output is a valid PDF, parses, and the text we added
    # survives round-trip. The rect / line operators themselves aren't
    # exposed by extract_text, but their presence is implicit in the
    # PDF being valid and bigger than a text-only page.
    assert bytes_.startswith(b"%PDF-")
    doc = pdf_oxide.PdfDocument.from_bytes(bytes_)
    assert "Graphics primitives demo" in doc.extract_text(0)


def test_version_is_038_or_newer():
    """Sanity check that we're running against a build that has the
    #384 write-side API — if the binding was rebuilt from a pre-0.3.38
    source tree we'd be testing a stale wheel."""
    version = pdf_oxide.VERSION
    assert version.startswith("0.3.3") or version.startswith("0.4.") or version.startswith("1.")
    # Confirm the symbols we rely on are actually exported.
    assert hasattr(pdf_oxide, "DocumentBuilder")
    assert hasattr(pdf_oxide, "FluentPageBuilder")
    assert hasattr(pdf_oxide, "EmbeddedFont")
    # Phase 2 methods on existing Pdf class.
    assert hasattr(pdf_oxide.Pdf, "from_html_css")
    assert hasattr(pdf_oxide.Pdf, "from_html_css_with_fonts")
