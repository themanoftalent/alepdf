<p align="center">
  <img src="assets/screenshots/alepdf-welcome.png" width="96" height="96" style="border-radius: 20px;" alt="AlePdf Icon">
</p>

<h1 align="center">AlePdf</h1>

<p align="center">
  <b>The modern, fast, private PDF editor and workbench built in pure Rust.</b><br>
  Read, edit, annotate, organize, combine, split, and protect PDFs in a blazing-fast native app.<br>
  <b>macOS · Windows · Linux · FreeBSD · Web</b>
</p>

<p align="center">
  <a href="https://alepdf-website.vercel.app/"><img alt="Official Website" src="https://img.shields.io/badge/website-alepdf--website.vercel.app-007acc?style=flat-square&logo=vercel"></a>
  <a href="https://github.com/themanoftalent/alepdf/releases"><img alt="Latest Release" src="https://img.shields.io/github/v/release/themanoftalent/alepdf?style=flat-square&color=008080"></a>
  <img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-b70228?style=flat-square">
  <img alt="Written in Rust" src="https://img.shields.io/badge/written%20in-Rust-8a011e?style=flat-square">
  <img alt="Privacy: No account, no telemetry" src="https://img.shields.io/badge/privacy-no%20telemetry%20%C2%B7%20local%20first-brightgreen?style=flat-square">
</p>

<p align="center">
  <a href="https://alepdf-website.vercel.app/"><b>Official Website</b></a> ·
  <a href="https://github.com/themanoftalent/alepdf/releases"><b>Download Releases</b></a> ·
  <a href="https://github.com/themanoftalent/alepdf/issues"><b>Report Issue</b></a>
</p>

<br>

<p align="center">
  <img src="assets/screenshots/alepdf-editor.png" alt="AlePdf in action: PDF editor and workspace" width="100%">
  <br>
  <sub>AlePdf native workspace: in-place PDF editing, threaded comments, and full document tools.</sub>
</p>

---

<p align="center">
  <a href="#highlights">Highlights</a> ·
  <a href="#features">Features</a> ·
  <a href="#downloads">Downloads</a> ·
  <a href="#built-for-agents--cli">Agents &amp; CLI</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href="#building-from-source">Building from Source</a> ·
  <a href="#license">License</a>
</p>

---

## Highlights

<table>
<tr>
<td width="33%" valign="top">

### ⚡ Blazing Fast &amp; Native
Engineered from the ground up in 100% pure Rust. Instant startup, silky-smooth scrolling, multi-threaded rendering, and minimal memory footprint.

</td>
<td width="33%" valign="top">

### 🛡️ Non-Destructive &amp; Safe
Atomic saves and incremental PDF updates ensure your original file bytes remain intact. Deep multi-level undo/redo and automatic crash recovery protect your work.

</td>
<td width="33%" valign="top">

### 🔒 100% Private &amp; Local
No mandatory accounts, no analytics, no telemetry, and no cloud processing. All document operations, OCR, and editing run entirely offline on your local device.

</td>
</tr>
</table>

---

## Features

### ✏️ In-Place PDF Text &amp; Image Editing
- Click directly on text blocks to edit wording, font attributes, sizing, alignment, and formatting.
- Integrated text reflow handles line wrapping, margins, and paragraph reorganizations naturally.
- Replace, move, resize, and rotate embedded images with live visual feedback.

### 📑 Visual Page Organization
- **Page Grid Overview**: See your entire document laid out like cards on a table.
- **Rearrange &amp; Duplicate**: Drag and drop to reorder pages, duplicate sections with Option/Ctrl-drag.
- **Rotate &amp; Delete**: Rotate portrait/landscape or delete unwanted pages in bulk.
- **Insert**: Inject blank pages or merge pages directly from external PDF documents.

### 💬 Rich Annotations &amp; Comments
- Markup text with highlights, underlines, strikethroughs, and squiggles.
- Insert sticky notes, callout boxes, shapes (rectangles, ellipses, arrows, polygons), and freehand ink drawings.
- Track review conversations in the dedicated threaded comments sidebar.

### 📝 Interactive Form Filling &amp; Creation
- Fill AcroForm and XFA fields seamlessly: text fields, multi-line boxes, checkboxes, radio buttons, dropdown lists, and combo boxes.
- Sign documents with digital signatures or handwritten ink signatures.
- Reset and clear forms or export form data with single-click actions.

### 🔀 Combine &amp; Split PDFs
- **Merge**: Combine multiple PDF files into a single master document, automatically organizing bookmarks into clean hierarchical trees.
- **Split**: Divide large documents into individual pages, chapters, or custom page number intervals.
- **Extract**: Isolate specific page selections into standalone PDF files without losing interactive form fields, layers, or attachments.

### 🔐 Security, Encryption &amp; Permissions
- Comprehensive PDF standard security handler: RC4 40/128-bit and AES-128 / AES-256 encryption.
- Respect and manage document permissions (printing, editing, copying, annotations).
- Permanent redaction and sanitization to scrub hidden metadata, scripts, and private data before distribution.

---

## Downloads

Download pre-built releases directly from our [Official Website](https://alepdf-website.vercel.app/) or [GitHub Releases](https://github.com/themanoftalent/alepdf/releases):

### macOS
- **Apple Silicon (M1/M2/M3/M4)**: [`AlePdf-0.18.2-macos-aarch64.dmg`](https://github.com/themanoftalent/alepdf/releases/tag/v0.18.2)
- Fast native execution, system file association, drag-and-drop installer.

### Windows
- **64-bit Windows (x64)**: Available via GitHub Releases (`.exe` / `.msi` / portable `.zip`).
- Full Explorer right-click context menu integration ("Edit with AlePdf").

### Linux &amp; Other Platforms
- Available as standalone binary, AppImage, and tarball builds across x86_64 and aarch64 architectures.

---

## Built for Agents &amp; CLI

AlePdf includes a powerful command-line interface (`pdfcraft-cli` / `alepdf-cli`) and built-in **Model Context Protocol (MCP)** support, allowing external scripts and AI agents (such as Claude, Codex, or local agents) to inspect, render, and modify PDFs programmatically:

```sh
# Inspect document structure as JSON
alepdf-cli info report.pdf

# Extract reading-order text from page 3
alepdf-cli text paper.pdf --page 3

# Headless batch editing: rotate pages and set metadata
alepdf-cli edit input.pdf --rotate 1,2:90 --delete 5 --title "Quarterly Report" --out output.pdf

# Merge multiple documents into one
alepdf-cli combine report.pdf appendix.pdf --out combined.pdf
```

### Model Context Protocol (MCP) Server
To let AI agents work with local documents safely, launch the opt-in MCP server:
```json
{
  "mcpServers": {
    "alepdf": {
      "command": "alepdf-cli",
      "args": ["mcp", "--root", "/path/to/pdfs"]
    }
  }
}
```

---

## Architecture

AlePdf is structured as a modular workspace of decoupled crates:

| Layer / Crate | Role |
|---|---|
| `pdfcraft-engine` | High-level facade for sessions, undo/redo stacks, tools, and document mutations. |
| `pdfcraft-ui-egui` | Desktop and web UI renderer, interactive canvases, toolbars, and mode state. |
| `pdfcraft-render` | High-fidelity page rasterization, font glyph rendering, and reading-order extraction. |
| `pdfcraft-organize` | Page geometry, tree manipulation, document merging, splitting, and outlines. |
| `pdfcraft-forms` | Interactive AcroForm and XFA field model, calculation engine, and appearance generation. |
| `pdfcraft-annot` | Comment appearances, text markup, vector drawing, stamps, and review threads. |
| `pdfcraft-crypt` | Standard security handler: RC4, AES-128/256, permissions, and cryptography. |
| `pdfcraft-cos` | Low-level tolerant PDF parser, repair heuristics, and incremental file writer. |
| `pdfcraft-automation` | Headless automation tool table, CLI operations, and MCP interface. |

---

## Building from Source

Building AlePdf requires **Rust 1.95 or later**:

```sh
# 1. Clone repository
git clone https://github.com/themanoftalent/alepdf.git
cd alepdf

# 2. Fetch OCR models (required for text recognition features)
cargo xtask models

# 3. Build and launch desktop application in release mode
cargo run --release -p pdfcraft -- sample.pdf
```

### Running Tests
```sh
cargo test --workspace
```

---

## Author &amp; Community

- **Creator &amp; Maintainer**: **TheManOfTalent**
- **Website**: [https://alepdf-website.vercel.app/](https://alepdf-website.vercel.app/)
- **Repository**: [https://github.com/themanoftalent/alepdf](https://github.com/themanoftalent/alepdf)

Feedback, feature requests, and contributions are welcome! Please open an issue on our [GitHub Issues](https://github.com/themanoftalent/alepdf/issues) page.

---

## License

AlePdf is open-source software dual-licensed under:
- **[MIT License](LICENSE-MIT)**
- **[Apache License 2.0](LICENSE-APACHE)**

at your option.

All bundled open-source fonts, icons, and dependencies retain their respective licenses, detailed in [ATTRIBUTION.md](ATTRIBUTION.md) and [NOTICE](NOTICE).
