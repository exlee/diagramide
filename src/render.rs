use crate::{OutputType, SPACE_MONO_NAME};

/// Render generated source into SVG using the editor's selected output
/// language.
pub fn render(output_type: OutputType, source: &str) -> Result<String, String> {
    let svg = match output_type {
        OutputType::Pikchr => {
            pikchr_pro::pikchr::render_pikchr(pikchr_pro::types::PikchrCode::new(source))
                .map(|svg| svg.into_inner())
                .map_err(|err| err.inner_string())?
        },
        OutputType::Svgbob => svgbob::to_svg_with_settings(
            source,
            &svgbob::Settings {
                font_family: SPACE_MONO_NAME.to_string(),
                ..Default::default()
            },
        ),
        OutputType::Text => text_to_svg(source),
    };

    Ok(inject_svg_style(&svg))
}

/// Font size of Text output, in SVG units.
const TEXT_FONT_SIZE: f32 = 14.0;
/// Space Mono advances 0.612 em per character.
const TEXT_ADVANCE: f32 = TEXT_FONT_SIZE * 0.612;
const TEXT_LINE_HEIGHT: f32 = TEXT_FONT_SIZE * 1.4;
const TEXT_MARGIN: f32 = 8.0;

/// Draw `source` as monospaced lines, unchanged.
fn text_to_svg(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let columns = lines.iter().map(|line| line.chars().count()).max().unwrap_or(0);
    let width = columns as f32 * TEXT_ADVANCE + 2.0 * TEXT_MARGIN;
    let height = lines.len() as f32 * TEXT_LINE_HEIGHT + 2.0 * TEXT_MARGIN;
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    );
    for (row, line) in lines.iter().enumerate() {
        let baseline = TEXT_MARGIN + (row as f32 + 0.8) * TEXT_LINE_HEIGHT;
        svg.push_str(&format!(
            r#"<text x="{TEXT_MARGIN}" y="{baseline}" font-size="{TEXT_FONT_SIZE}" fill="black" xml:space="preserve">{}</text>"#,
            escape_xml(line)
        ));
    }
    svg.push_str("</svg>");
    svg
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Apply the application's diagram font to either renderer's SVG output.
pub fn inject_svg_style(svg: &str) -> String {
    let mut output = svg.to_owned();
    if let Some(index) = output.find('>') {
        let style = format!(
            "<style>text,path {{ font-family: '{}'; }}</style>",
            SPACE_MONO_NAME
        );
        output.insert_str(index + 1, &style);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_pikchr_and_svgbob() {
        let pikchr = render(OutputType::Pikchr, "box").unwrap();
        assert!(pikchr.starts_with("<svg"));
        assert!(pikchr.contains("Space Mono"));

        let svgbob = render(OutputType::Svgbob, "+---+\n| A |\n+---+").unwrap();
        assert!(svgbob.starts_with("<svg"));
        assert!(svgbob.contains("Space Mono"));
    }

    #[test]
    fn text_output_keeps_every_line_and_escapes_markup() {
        let svg = render(OutputType::Text, "(fact a)\n  <b> & c").unwrap();
        assert_eq!(svg.matches("<text ").count(), 2);
        assert!(svg.contains(">(fact a)</text>"));
        assert!(svg.contains(">  &lt;b&gt; &amp; c</text>"));
        assert!(svg.contains("Space Mono"));
    }

    #[test]
    fn reports_pikchr_errors() {
        let error = render(OutputType::Pikchr, "not valid pikchr {").unwrap_err();
        assert!(!error.is_empty());
    }
}
