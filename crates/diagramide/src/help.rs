use eframe::egui;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HelpTopic {
    #[default]
    Overview,
    Pikchr,
    Prolog,
    Tcl,
    Mruby,
    PlainText,
    Render,
}

impl HelpTopic {
    fn title(self) -> &'static str {
        match self {
            Self::Overview => "DiagramIDE Help",
            Self::Pikchr => "Pikchr Editor Help",
            Self::Prolog => "Prolog Editor Help",
            Self::Tcl => "Tcl Editor Help",
            Self::Mruby => "mruby Editor Help",
            Self::PlainText => "Plain Text Help",
            Self::Render => "Render Window Help",
        }
    }
}

fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.heading(text);
}

fn feature(ui: &mut egui::Ui, name: &str, description: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(name).strong());
        ui.label(description);
    });
}

fn common_editor_help(ui: &mut egui::Ui) {
    heading(ui, "Editing");
    feature(
        ui,
        "Live updates",
        "The render and dependent windows update automatically after you edit source.",
    );
    feature(
        ui,
        "Cmd/Ctrl+R",
        "Renames the focused editor. Names are used by cross-window references.",
    );
    feature(
        ui,
        "Enter",
        "Inserts a newline and automatically carries or adjusts indentation.",
    );
    feature(
        ui,
        "Close button",
        "Hides the window. Reopen it from Windows in the main menu.",
    );
    feature(
        ui,
        "Cmd/Ctrl + close button",
        "Permanently deletes the editor and its render window from the workspace.",
    );

    heading(ui, "Cross-window references");
    feature(
        ui,
        "!!NAME!!",
        "Includes the raw source text of another named editor. Plain text windows can be included this way.",
    );
    feature(
        ui,
        "$$NAME$$",
        "Includes the generated Pikchr output of another named diagram editor.",
    );
    ui.label("References can be nested up to three replacement passes.");
}

fn topic_help(ui: &mut egui::Ui, topic: HelpTopic) {
    match topic {
        HelpTopic::Overview => {},
        HelpTopic::Pikchr => {
            common_editor_help(ui);
            heading(ui, "Pikchr");
            ui.label(
                "Write Pikchr directly. Valid source is rendered live in the paired Render window.",
            );
        },
        HelpTopic::Prolog => {
            common_editor_help(ui);
            heading(ui, "Prolog");
            ui.label("Define a diagram//0 DCG. Its text output is interpreted as Pikchr.");
        },
        HelpTopic::Tcl => {
            common_editor_help(ui);
            heading(ui, "Tcl");
            ui.label("Return a string containing Pikchr source. The Tcl editor is available only when Tcl 8.6 can be loaded.");
        },
        HelpTopic::Mruby => {
            common_editor_help(ui);
            heading(ui, "mruby");
            ui.label("Text written with print or puts becomes Pikchr source. The editor is available only when the mruby executable is found.");
        },
        HelpTopic::PlainText => {
            common_editor_help(ui);
            heading(ui, "Plain text");
            ui.label("Stores reusable raw text. Include it from another editor with !!NAME!!; it has no generated Pikchr output or Render window.");
        },
        HelpTopic::Render => {
            heading(ui, "Render window");
            feature(
                ui,
                "Automatic preview",
                "Displays the paired editor's generated Pikchr output and redraws as the window is resized.",
            );
            feature(
                ui,
                "Export",
                "Exports SVG, PNG, transparent PNG, or copies generated Pikchr code to the clipboard.",
            );
            feature(
                ui,
                "Close button",
                "Hides the preview. Reopen it from Windows in the main menu.",
            );
            feature(
                ui,
                "Cmd/Ctrl + close button",
                "Permanently deletes only this Render window. It is recreated when its editor next renders.",
            );
        },
    }
}

pub fn window(ctx: &egui::Context, topic: HelpTopic) -> bool {
    let mut open = true;
    egui::Window::new(topic.title())
        .id(egui::Id::new("diagramide_help"))
        .open(&mut open)
        .default_size((520.0, 560.0))
        .resizable(true)
        .show(ctx, |ui| {
            ui.style_mut().override_font_id = Some(egui::TextStyle::Monospace.resolve(ui.style()));
            egui::ScrollArea::vertical().show(ui, |ui| {
                if topic != HelpTopic::Overview {
                    topic_help(ui, topic);
                    ui.separator();
                    heading(ui, "Full feature guide");
                }

                heading(ui, "Workspace");
                feature(
                    ui,
                    "Autosave",
                    "The current workspace and window layout persist between app launches.",
                );
                feature(
                    ui,
                    "Save / Load Workspace",
                    "Exports or imports the complete workspace as JSON.",
                );
                feature(
                    ui,
                    "Reset Workspace",
                    "Deletes all workspace windows after confirmation.",
                );
                feature(
                    ui,
                    "Windows",
                    "Shows or hides existing windows, plus the diagnostic Logger and Debug windows.",
                );
                feature(
                    ui,
                    "View",
                    "Changes the scale of the complete interface.",
                );

                common_editor_help(ui);

                heading(ui, "Editor types");
                feature(ui, "Pikchr", "Direct Pikchr source with live rendering.");
                feature(ui, "Prolog", "A diagram//0 DCG generates Pikchr source.");
                feature(
                    ui,
                    "Tcl",
                    "A Tcl script returns Pikchr source when Tcl 8.6 is available.",
                );
                feature(
                    ui,
                    "mruby",
                    "print and puts output becomes Pikchr when mruby is available.",
                );
                feature(
                    ui,
                    "Plain text",
                    "Reusable raw text for !!NAME!! references; no paired render.",
                );

                heading(ui, "Rendering and export");
                feature(
                    ui,
                    "Live Render window",
                    "Diagram editors own a paired, resizable preview window.",
                );
                feature(
                    ui,
                    "Export",
                    "Render windows export SVG, PNG, transparent PNG, and generated Pikchr source.",
                );
                feature(
                    ui,
                    "Errors",
                    "Evaluation and rendering errors appear next to their editor and in the Logger.",
                );
            });
        });
    open
}

#[cfg(test)]
mod tests {
    use super::HelpTopic;

    #[test]
    fn help_topic_defaults_to_overview() {
        assert_eq!(HelpTopic::default(), HelpTopic::Overview);
    }
}
