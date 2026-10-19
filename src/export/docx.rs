//! Markdown -> Word (.docx) conversion.
//!
//! The converter walks the CommonMark AST and maps the common block and inline
//! constructs to native Word paragraphs, runs, hyperlinks and tables. Images
//! are represented by a textual placeholder in this first version.

use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use comrak::nodes::{
    AstNode, ListType as MdListType, NodeCode, NodeCodeBlock, NodeLink, NodeList, NodeTaskItem,
    NodeValue, TableAlignment as MdTableAlignment,
};
use comrak::{Arena, parse_document};
use docx_rs::*;

use url::Url;

use crate::export::ExportError;

const A4_WIDTH_TWIPS: usize = 11_906;
const A4_HEIGHT_TWIPS: usize = 16_838;
const PAGE_MARGIN_TWIPS: i32 = 1_440;
const CONTENT_WIDTH_TWIPS: usize = A4_WIDTH_TWIPS - (PAGE_MARGIN_TWIPS as usize * 2);
const BODY_SIZE: usize = 22; // half-points, 11 pt
const BODY_FONT: &str = "Segoe UI";
const EAST_ASIA_FONT: &str = "Microsoft YaHei";
const MONO_FONT: &str = "Consolas";

/// Export Markdown directly to a Word document.
pub fn export_docx(
    markdown: &str,
    output: &Path,
    source_dir: Option<&Path>,
) -> Result<(), ExportError> {
    let bytes = markdown_to_docx_bytes(markdown, source_dir)?;
    fs::write(output, bytes)?;
    Ok(())
}

/// Build the .docx archive in memory (useful for tests and future features).
pub fn markdown_to_docx_bytes(
    markdown: &str,
    source_dir: Option<&Path>,
) -> Result<Vec<u8>, ExportError> {
    let options = crate::markdown::options();
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options);

    let mut footnote_numbers = HashMap::new();
    for node in root.descendants() {
        if let NodeValue::FootnoteReference(reference) = &node.data.borrow().value {
            footnote_numbers
                .entry(reference.name.clone())
                .or_insert(reference.ix);
        }
    }

    let mut renderer = Renderer {
        blocks: Vec::new(),
        footnote_numbers,
        source_dir: source_dir.map(Path::to_path_buf),
    };
    let context = BlockContext::default();
    let mut footnotes = Vec::new();

    for node in root.children() {
        if matches!(node.data.borrow().value, NodeValue::FootnoteDefinition(_)) {
            footnotes.push(node);
        } else {
            renderer.render_block(node, &context);
        }
    }

    if !footnotes.is_empty() {
        renderer.render_footnotes(&footnotes);
    }

    let mut docx = Docx::new()
        .page_size(A4_WIDTH_TWIPS as u32, A4_HEIGHT_TWIPS as u32)
        .page_margin(PageMargin {
            top: PAGE_MARGIN_TWIPS,
            left: PAGE_MARGIN_TWIPS,
            bottom: PAGE_MARGIN_TWIPS,
            right: PAGE_MARGIN_TWIPS,
            header: 720,
            footer: 720,
            gutter: 0,
        })
        .default_fonts(body_fonts())
        .default_size(BODY_SIZE)
        .default_line_spacing(
            LineSpacing::new()
                .line_rule(LineSpacingType::Auto)
                .line(300)
                .after(100),
        );

    let ends_with_table = matches!(renderer.blocks.last(), Some(Block::Table(_)));
    for block in renderer.blocks {
        docx = match block {
            Block::Paragraph(paragraph) => docx.add_paragraph(paragraph),
            Block::Table(table) => docx.add_table(table),
        };
    }
    if ends_with_table {
        // Word prefers a paragraph after a trailing table.
        docx = docx.add_paragraph(Paragraph::new());
    }

    let mut cursor = Cursor::new(Vec::new());
    docx.build()
        .pack(&mut cursor)
        .map_err(|error| ExportError::Docx(error.to_string()))?;
    Ok(cursor.into_inner())
}

#[derive(Clone, Debug, Default)]
struct BlockContext {
    indent: i32,
    color: Option<String>,
    compact: bool,
    quote: bool,
}

struct Renderer {
    blocks: Vec<Block>,
    footnote_numbers: HashMap<String, u32>,
    source_dir: Option<PathBuf>,
}

#[allow(clippy::large_enum_variant)]
enum Block {
    Paragraph(Paragraph),
    Table(Table),
}

#[allow(clippy::large_enum_variant)]
enum InlineOut {
    Run(Run),
    Link(Hyperlink),
}

#[derive(Clone)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
    underline: bool,
    highlight: bool,
    color: Option<String>,
    size: usize,
    vert_align: Option<VertAlignType>,
}

impl Default for InlineStyle {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            strike: false,
            code: false,
            underline: false,
            highlight: false,
            color: None,
            size: BODY_SIZE,
            vert_align: None,
        }
    }
}

impl Renderer {
    fn render_block<'a>(&mut self, node: &'a AstNode<'a>, context: &BlockContext) {
        let value = node.data.borrow().value.clone();
        match value {
            NodeValue::Document => {
                for child in node.children() {
                    self.render_block(child, context);
                }
            }
            NodeValue::Paragraph => {
                self.render_paragraph(node, context, None);
            }
            NodeValue::Heading(heading) => {
                self.render_heading(node, context, heading.level);
            }
            NodeValue::CodeBlock(code) => {
                self.render_code_block(&code, context);
            }
            NodeValue::BlockQuote | NodeValue::MultilineBlockQuote(_) => {
                let quoted = BlockContext {
                    indent: context.indent + 360,
                    color: Some("4E5964".to_owned()),
                    compact: false,
                    quote: true,
                };
                for child in node.children() {
                    self.render_block(child, &quoted);
                }
            }
            NodeValue::Alert(alert) => {
                let (label, color) = alert_label(alert.alert_type);
                let title = alert.title.clone().unwrap_or_else(|| label.to_owned());
                let mut paragraph = self.base_paragraph(context, false);
                let style = InlineStyle {
                    bold: true,
                    color: Some(color.to_owned()),
                    size: BODY_SIZE,
                    ..InlineStyle::default()
                };
                paragraph = paragraph.add_run(styled_run(&title, &style));
                self.blocks.push(Block::Paragraph(paragraph));

                let quoted = BlockContext {
                    indent: context.indent + 360,
                    color: Some("4E5964".to_owned()),
                    compact: false,
                    quote: true,
                };
                for child in node.children() {
                    self.render_block(child, &quoted);
                }
            }
            NodeValue::List(list) => {
                self.render_list(node, list, context);
            }
            NodeValue::DescriptionList => {
                for child in node.children() {
                    self.render_block(child, context);
                }
            }
            NodeValue::DescriptionItem(_) => {
                for child in node.children() {
                    self.render_block(child, context);
                }
            }
            NodeValue::DescriptionTerm => {
                let style = InlineStyle {
                    bold: true,
                    color: context.color.clone(),
                    size: 23,
                    ..InlineStyle::default()
                };
                let mut paragraph = self.base_paragraph(context, false);
                for inline in self.render_inlines(node, &style) {
                    add_inline(&mut paragraph, inline);
                }
                self.blocks.push(Block::Paragraph(paragraph));
            }
            NodeValue::DescriptionDetails => {
                let detail = BlockContext {
                    indent: context.indent + 360,
                    color: Some("4E5964".to_owned()),
                    compact: false,
                    quote: false,
                };
                for child in node.children() {
                    self.render_block(child, &detail);
                }
            }
            NodeValue::Table(table) => {
                self.render_table(node, &table, context);
            }
            NodeValue::ThematicBreak => {
                let style = InlineStyle {
                    color: Some("8C959F".to_owned()),
                    size: BODY_SIZE,
                    ..InlineStyle::default()
                };
                let paragraph = Paragraph::new()
                    .add_run(styled_run("·   ·   ·", &style))
                    .align(AlignmentType::Center)
                    .line_spacing(LineSpacing::new().before(120).after(120));
                self.blocks.push(Block::Paragraph(paragraph));
            }
            NodeValue::HtmlBlock(_) | NodeValue::FrontMatter(_) => {}
            NodeValue::FootnoteDefinition(_) => {}
            NodeValue::Raw(raw) => {
                if !raw.trim().is_empty() {
                    self.blocks.push(Block::Paragraph(
                        self.base_paragraph(context, false)
                            .add_run(styled_run(&raw, &InlineStyle::default())),
                    ));
                }
            }
            _ => {
                // Fallback for container nodes such as table rows or future
                // Comrak variants.
                let has_children = node.children().next().is_some();
                if has_children {
                    for child in node.children() {
                        self.render_block(child, context);
                    }
                }
            }
        }
    }

    fn render_paragraph<'a>(
        &mut self,
        node: &'a AstNode<'a>,
        context: &BlockContext,
        prefix: Option<&str>,
    ) {
        let style = InlineStyle {
            size: BODY_SIZE,
            color: context.color.clone(),
            ..InlineStyle::default()
        };
        let mut paragraph = self.base_paragraph(context, prefix.is_some());
        if let Some(prefix) = prefix {
            paragraph = paragraph.add_run(styled_run(prefix, &style));
        }
        for inline in self.render_inlines(node, &style) {
            add_inline(&mut paragraph, inline);
        }
        self.blocks.push(Block::Paragraph(paragraph));
    }

    fn render_heading<'a>(&mut self, node: &'a AstNode<'a>, context: &BlockContext, level: u8) {
        let level = level.clamp(1, 6);
        let size = match level {
            1 => 40,
            2 => 32,
            3 => 28,
            4 => 25,
            5 => 23,
            _ => 22,
        };
        let color = if level >= 6 { "57606A" } else { "1F2328" };
        let before = if level == 1 { 0 } else { 260 };
        let after = if level <= 2 { 160 } else { 120 };

        let style = InlineStyle {
            bold: true,
            size,
            color: Some(color.to_owned()),
            ..InlineStyle::default()
        };

        let mut paragraph = Paragraph::new()
            .line_spacing(LineSpacing::new().before(before).after(after))
            .keep_next(true)
            .outline_lvl(level as usize);
        if context.indent > 0 {
            paragraph = paragraph.indent(Some(context.indent), None, None, None);
        }

        for inline in self.render_inlines(node, &style) {
            add_inline(&mut paragraph, inline);
        }
        self.blocks.push(Block::Paragraph(paragraph));
    }

    fn render_code_block(&mut self, code: &NodeCodeBlock, context: &BlockContext) {
        let style = InlineStyle {
            code: true,
            size: 18,
            color: Some("24292F".to_owned()),
            ..InlineStyle::default()
        };

        let literal = code.literal.strip_suffix('\n').unwrap_or(&code.literal);
        let mut run = Run::new();
        for (index, line) in literal.split('\n').enumerate() {
            if index > 0 {
                run = run.add_break(BreakType::TextWrapping);
            }
            if !line.is_empty() {
                run = run.add_text(line);
            }
        }
        apply_run_style(&mut run, &style);

        let mut paragraph = Paragraph::new()
            .line_spacing(LineSpacing::new().line(260).after(120))
            .keep_lines(true)
            .indent(Some(context.indent + 180), None, Some(180), None)
            .add_run(run);
        decorate_code_block(&mut paragraph);
        self.blocks.push(Block::Paragraph(paragraph));
    }

    fn render_list<'a>(&mut self, node: &'a AstNode<'a>, list: NodeList, context: &BlockContext) {
        for (index, item) in node.children().enumerate() {
            let item_value = item.data.borrow().value.clone();
            let prefix = match &item_value {
                NodeValue::TaskItem(task) => {
                    format!("{}  ", task_glyph(task))
                }
                _ => match list.list_type {
                    MdListType::Bullet => {
                        let depth = (context.indent / 360).max(0) as usize;
                        let glyph = match depth % 3 {
                            0 => "•",
                            1 => "◦",
                            _ => "▪",
                        };
                        format!("{glyph}  ")
                    }
                    MdListType::Ordered => {
                        format!("{}.  ", list.start + index)
                    }
                },
            };

            let item_context = BlockContext {
                indent: context.indent + 360,
                color: context.color.clone(),
                compact: list.tight,
                quote: context.quote,
            };

            let mut prefix_used = false;
            for child in item.children() {
                if !prefix_used && matches!(child.data.borrow().value, NodeValue::Paragraph) {
                    self.render_paragraph(child, &item_context, Some(&prefix));
                    prefix_used = true;
                } else {
                    self.render_block(child, &item_context);
                }
            }

            if !prefix_used {
                let style = InlineStyle {
                    color: context.color.clone(),
                    size: BODY_SIZE,
                    ..InlineStyle::default()
                };
                let paragraph = self
                    .base_paragraph(&item_context, true)
                    .add_run(styled_run(&prefix, &style));
                self.blocks.push(Block::Paragraph(paragraph));
            }
        }
    }

    fn render_table<'a>(
        &mut self,
        node: &'a AstNode<'a>,
        table: &comrak::nodes::NodeTable,
        context: &BlockContext,
    ) {
        let column_count = table.num_columns.max(1);
        let column_width = CONTENT_WIDTH_TWIPS / column_count;

        let mut rows = Vec::new();
        for row_node in node.children() {
            let is_header = matches!(row_node.data.borrow().value, NodeValue::TableRow(true));
            let mut cells = Vec::new();

            for (column, cell_node) in row_node.children().enumerate() {
                let align = table
                    .alignments
                    .get(column)
                    .copied()
                    .unwrap_or(MdTableAlignment::None);
                let style = InlineStyle {
                    bold: is_header,
                    size: 21,
                    color: context.color.clone(),
                    ..InlineStyle::default()
                };

                let mut paragraph = Paragraph::new()
                    .line_spacing(LineSpacing::new().line(260).after(20))
                    .align(docx_alignment(align));
                for inline in self.render_inlines(cell_node, &style) {
                    add_inline(&mut paragraph, inline);
                }

                let mut cell = TableCell::new()
                    .add_paragraph(paragraph)
                    .width(column_width, WidthType::Dxa);
                if is_header {
                    cell = cell.shading(Shading::new().fill("F0F4F3"));
                }
                cells.push(cell);
            }

            while cells.len() < column_count {
                cells.push(
                    TableCell::new()
                        .add_paragraph(Paragraph::new())
                        .width(column_width, WidthType::Dxa),
                );
            }

            let mut row = TableRow::new(cells);
            if is_header {
                row = row.cant_split();
            }
            rows.push(row);
        }

        if rows.is_empty() {
            return;
        }

        let table = Table::new(rows)
            .set_grid(vec![column_width; column_count])
            .width(CONTENT_WIDTH_TWIPS, WidthType::Dxa)
            .layout(TableLayoutType::Fixed)
            .margins(TableCellMargins::new().margin(60, 100, 60, 100))
            .set_borders(table_borders());
        self.blocks.push(Block::Table(table));
    }

    fn render_footnotes<'a>(&mut self, notes: &[&'a AstNode<'a>]) {
        let style = InlineStyle {
            bold: true,
            size: 24,
            color: Some("1F2328".to_owned()),
            ..InlineStyle::default()
        };
        let heading = Paragraph::new()
            .line_spacing(LineSpacing::new().before(260).after(100))
            .keep_next(true)
            .add_run(styled_run("脚注", &style));
        self.blocks.push(Block::Paragraph(heading));

        for (index, node) in notes.iter().enumerate() {
            let name = match &node.data.borrow().value {
                NodeValue::FootnoteDefinition(definition) => definition.name.clone(),
                _ => continue,
            };
            let number = self
                .footnote_numbers
                .get(&name)
                .copied()
                .unwrap_or((index + 1) as u32)
                .max(1);
            let context = BlockContext {
                indent: 360,
                color: Some("4E5964".to_owned()),
                compact: false,
                quote: false,
            };
            let prefix = format!("[{number}]  ");
            let mut used_prefix = false;

            for child in node.children() {
                if !used_prefix && matches!(child.data.borrow().value, NodeValue::Paragraph) {
                    self.render_paragraph(child, &context, Some(&prefix));
                    used_prefix = true;
                } else {
                    self.render_block(child, &context);
                }
            }

            if !used_prefix {
                let style = InlineStyle {
                    color: Some("4E5964".to_owned()),
                    size: 20,
                    ..InlineStyle::default()
                };
                self.blocks.push(Block::Paragraph(
                    self.base_paragraph(&context, true)
                        .add_run(styled_run(&prefix, &style)),
                ));
            }
        }
    }

    fn base_paragraph(&self, context: &BlockContext, hanging: bool) -> Paragraph {
        let after = if context.compact { 40 } else { 100 };
        let mut paragraph = Paragraph::new().line_spacing(
            LineSpacing::new()
                .line_rule(LineSpacingType::Auto)
                .line(300)
                .after(after),
        );

        if hanging {
            paragraph = paragraph.indent(
                Some(context.indent),
                Some(SpecialIndentType::Hanging(320)),
                None,
                None,
            );
        } else if context.indent > 0 {
            paragraph = paragraph.indent(Some(context.indent), None, None, None);
        }

        if context.quote {
            decorate_quote(&mut paragraph);
        }

        paragraph
    }

    fn image_run(&self, link: &NodeLink) -> Option<Run> {
        let path = self.resolve_image_path(&link.url)?;
        let bytes = fs::read(&path).ok()?;
        let (width, height) = image::image_dimensions(&path).ok()?;
        if width == 0 || height == 0 {
            return None;
        }

        const MAX_WIDTH_PX: f32 = 560.0;
        const MAX_HEIGHT_PX: f32 = 440.0;
        let scale = (MAX_WIDTH_PX / width as f32)
            .min(MAX_HEIGHT_PX / height as f32)
            .min(1.0);
        let render_width = (width as f32 * scale).round().max(1.0) as u32;
        let render_height = (height as f32 * scale).round().max(1.0) as u32;
        let emu_per_px = 9_525u32;

        let pic = Pic::new(&bytes).size(render_width * emu_per_px, render_height * emu_per_px);
        Some(Run::new().add_image(pic))
    }

    fn resolve_image_path(&self, url: &str) -> Option<PathBuf> {
        if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("data:") {
            return None;
        }

        if url.starts_with("file:") {
            return Url::parse(url).ok()?.to_file_path().ok();
        }

        let decoded = percent_encoding::percent_decode_str(url)
            .decode_utf8()
            .ok()?;
        let path = PathBuf::from(decoded.as_ref());
        if path.is_absolute() {
            Some(path)
        } else {
            Some(self.source_dir.as_deref()?.join(path))
        }
    }

    fn render_inlines<'a>(&self, parent: &'a AstNode<'a>, style: &InlineStyle) -> Vec<InlineOut> {
        let mut output = Vec::new();
        for child in parent.children() {
            let value = child.data.borrow().value.clone();
            match value {
                NodeValue::Text(text) => {
                    output.push(InlineOut::Run(styled_run(&text, style)));
                }
                NodeValue::SoftBreak => {
                    output.push(InlineOut::Run(styled_run(" ", style)));
                }
                NodeValue::LineBreak => {
                    let mut run = Run::new();
                    run = run.add_break(BreakType::TextWrapping);
                    apply_run_style(&mut run, style);
                    output.push(InlineOut::Run(run));
                }
                NodeValue::Code(NodeCode { literal, .. }) => {
                    let code_style = InlineStyle {
                        code: true,
                        ..style.clone()
                    };
                    output.push(InlineOut::Run(styled_run(&literal, &code_style)));
                }
                NodeValue::Strong => {
                    let nested = InlineStyle {
                        bold: true,
                        ..style.clone()
                    };
                    output.extend(self.render_inlines(child, &nested));
                }
                NodeValue::Emph => {
                    let nested = InlineStyle {
                        italic: true,
                        ..style.clone()
                    };
                    output.extend(self.render_inlines(child, &nested));
                }
                NodeValue::Strikethrough => {
                    let nested = InlineStyle {
                        strike: true,
                        ..style.clone()
                    };
                    output.extend(self.render_inlines(child, &nested));
                }
                NodeValue::Underline => {
                    let nested = InlineStyle {
                        underline: true,
                        ..style.clone()
                    };
                    output.extend(self.render_inlines(child, &nested));
                }
                NodeValue::Highlight => {
                    let nested = InlineStyle {
                        highlight: true,
                        ..style.clone()
                    };
                    output.extend(self.render_inlines(child, &nested));
                }
                NodeValue::Superscript => {
                    let nested = InlineStyle {
                        vert_align: Some(VertAlignType::SuperScript),
                        ..style.clone()
                    };
                    output.extend(self.render_inlines(child, &nested));
                }
                NodeValue::Subscript => {
                    let nested = InlineStyle {
                        vert_align: Some(VertAlignType::SubScript),
                        ..style.clone()
                    };
                    output.extend(self.render_inlines(child, &nested));
                }
                NodeValue::Link(link) => {
                    let link_style = InlineStyle {
                        color: Some("0B628C".to_owned()),
                        underline: true,
                        ..style.clone()
                    };
                    let runs = self.render_inlines(child, &link_style);

                    if is_external_link(&link.url) {
                        let mut hyperlink =
                            Hyperlink::new(link.url.clone(), HyperlinkType::External);
                        let mut has_text = false;
                        for run in runs {
                            if let InlineOut::Run(run) = run {
                                hyperlink = hyperlink.add_run(run);
                                has_text = true;
                            }
                        }
                        if !has_text {
                            hyperlink = hyperlink.add_run(styled_run(&link.url, &link_style));
                        }
                        output.push(InlineOut::Link(hyperlink));
                    } else {
                        output.extend(runs);
                    }
                }
                NodeValue::Image(link) => {
                    if let Some(run) = self.image_run(&link) {
                        output.push(InlineOut::Run(run));
                    } else {
                        let alt = collect_text(child);
                        let label = if alt.trim().is_empty() {
                            "[图片]".to_owned()
                        } else {
                            format!("[图片：{}]", alt.trim())
                        };
                        let image_style = InlineStyle {
                            italic: true,
                            color: Some("57606A".to_owned()),
                            ..style.clone()
                        };
                        output.push(InlineOut::Run(styled_run(&label, &image_style)));
                    }
                }
                NodeValue::FootnoteReference(reference) => {
                    let number = reference.ix.max(1);
                    let footnote_style = InlineStyle {
                        vert_align: Some(VertAlignType::SuperScript),
                        size: 16,
                        color: Some("0B628C".to_owned()),
                        ..style.clone()
                    };
                    output.push(InlineOut::Run(styled_run(
                        &format!("[{number}]"),
                        &footnote_style,
                    )));
                }
                NodeValue::HtmlInline(_) => {}
                NodeValue::Math(math) => {
                    let math_style = InlineStyle {
                        code: true,
                        ..style.clone()
                    };
                    output.push(InlineOut::Run(styled_run(&math.literal, &math_style)));
                }
                _ => {
                    if child.children().next().is_some() {
                        output.extend(self.render_inlines(child, style));
                    }
                }
            }
        }
        output
    }
}

fn add_inline(paragraph: &mut Paragraph, inline: InlineOut) {
    let placeholder = Paragraph::new();
    let taken = std::mem::replace(paragraph, placeholder);
    *paragraph = match inline {
        InlineOut::Run(run) => taken.add_run(run),
        InlineOut::Link(link) => taken.add_hyperlink(link),
    };
}

fn docx_alignment(alignment: MdTableAlignment) -> AlignmentType {
    match alignment {
        MdTableAlignment::Left => AlignmentType::Left,
        MdTableAlignment::Center => AlignmentType::Center,
        MdTableAlignment::Right => AlignmentType::Right,
        MdTableAlignment::None => AlignmentType::Left,
    }
}

fn task_glyph(task: &NodeTaskItem) -> &'static str {
    if task.symbol.is_some() { "☑" } else { "☐" }
}

fn alert_label(alert: comrak::nodes::AlertType) -> (&'static str, &'static str) {
    use comrak::nodes::AlertType;
    match alert {
        AlertType::Note => ("提示", "0969DA"),
        AlertType::Tip => ("技巧", "1A7F37"),
        AlertType::Important => ("重要", "8250DF"),
        AlertType::Warning => ("警告", "9A6700"),
        AlertType::Caution => ("注意", "CF222E"),
    }
}

fn decorate_code_block(paragraph: &mut Paragraph) {
    paragraph.property.shading = Some(Shading::new().fill("F6F8FA"));
    let mut borders = ParagraphBorders::with_empty();
    for position in [
        ParagraphBorderPosition::Top,
        ParagraphBorderPosition::Left,
        ParagraphBorderPosition::Bottom,
        ParagraphBorderPosition::Right,
    ] {
        borders = borders.set(
            ParagraphBorder::new(position)
                .color("DDE3E8")
                .size(4)
                .space(4),
        );
    }
    paragraph.property.borders = Some(borders);
}

fn decorate_quote(paragraph: &mut Paragraph) {
    paragraph.property.borders = Some(
        ParagraphBorders::with_empty().set(
            ParagraphBorder::new(ParagraphBorderPosition::Left)
                .color("B7C7C2")
                .size(18)
                .space(8),
        ),
    );
    paragraph.property.shading = Some(Shading::new().fill("FAFBFB"));
}

fn table_borders() -> TableBorders {
    let positions = [
        TableBorderPosition::Top,
        TableBorderPosition::Left,
        TableBorderPosition::Bottom,
        TableBorderPosition::Right,
        TableBorderPosition::InsideH,
        TableBorderPosition::InsideV,
    ];
    let mut borders = TableBorders::new();
    for position in positions {
        borders = borders.set(TableBorder::new(position).color("D5DCE1").size(4));
    }
    borders
}

fn is_external_link(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:")
}

fn body_fonts() -> RunFonts {
    RunFonts::new()
        .ascii(BODY_FONT)
        .hi_ansi(BODY_FONT)
        .east_asia(EAST_ASIA_FONT)
        .cs(BODY_FONT)
}

fn mono_fonts() -> RunFonts {
    RunFonts::new()
        .ascii(MONO_FONT)
        .hi_ansi(MONO_FONT)
        .east_asia(EAST_ASIA_FONT)
        .cs(MONO_FONT)
}

fn styled_run(text: &str, style: &InlineStyle) -> Run {
    let mut run = Run::new();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            run = run.add_break(BreakType::TextWrapping);
        }
        if !line.is_empty() {
            run = run.add_text(line);
        }
    }
    apply_run_style(&mut run, style);
    run
}

fn apply_run_style(run: &mut Run, style: &InlineStyle) {
    let mut property = run.run_property.clone();
    property = property
        .fonts(if style.code {
            mono_fonts()
        } else {
            body_fonts()
        })
        .size(style.size);

    if style.bold {
        property = property.bold();
    }
    if style.italic {
        property = property.italic();
    }
    if style.strike {
        property = property.strike();
    }
    if style.underline {
        property = property.underline("single");
    }
    if style.highlight {
        property = property.highlight("yellow");
    }
    if let Some(color) = &style.color {
        property = property.color(color.clone());
    }
    if style.code {
        property = property.shading(Shading::new().fill("F2F4F6"));
    }
    if let Some(value) = style.vert_align {
        property = property.vert_align(value);
    }

    run.run_property = property;
}

fn collect_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    for child in node.children() {
        let value = child.data.borrow().value.clone();
        match value {
            NodeValue::Text(inner) => text.push_str(&inner),
            NodeValue::Code(code) => text.push_str(&code.literal),
            NodeValue::Math(math) => text.push_str(&math.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => text.push(' '),
            _ => {
                if child.children().next().is_some() {
                    text.push_str(&collect_text(child));
                }
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_docx_archive() {
        let bytes = markdown_to_docx_bytes(
            "# 标题\n\n这是一段 **粗体**、*斜体*、`代码` 和 [链接](https://example.com)。\n\n\
             - 第一项\n- [x] 已完成\n\n> 引用\n\n| 列 A | 列 B |\n| :-- | --: |\n| 1 | 2 |\n\n\
             ```rust\nfn main() {}\n```\n",
            None,
        )
        .expect("docx generation failed");
        assert!(bytes.starts_with(b"PK"));
        assert!(bytes.len() > 2_000);
    }

    #[test]
    fn embeds_local_images() {
        let source_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let bytes = markdown_to_docx_bytes("![MarkLite](banner.png)", Some(&source_dir))
            .expect("docx generation failed");
        assert!(bytes.starts_with(b"PK"));
        assert!(
            bytes
                .windows(b"word/media".len())
                .any(|window| window == b"word/media")
        );
    }

    #[test]
    fn exports_to_file() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("mdlite-test-{}.docx", std::process::id()));
        export_docx("# Hello\n\nWorld", &path, None).expect("export failed");
        let metadata = fs::metadata(&path).expect("missing docx");
        assert!(metadata.len() > 1_000);
        let _ = fs::remove_file(path);
    }
}
