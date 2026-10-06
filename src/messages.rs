use std::time::Duration;

use eframe::egui::{self, Context};

use crate::{help::HelpTopic, mini_window::WindowType, state};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum ExportType {
    Svg,
    Png,
    PngTransparent,
    Source(SourceFormat),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub enum Msg {
    Batch(Vec<Msg>),
    Debounce(Duration, egui::Id, Box<Msg>),
    PopModal,
    CheckDependencies,
    ShowHelp(HelpTopic),
    SelectTheme(#[serde(skip)] Context, String),
    ReloadThemes(#[serde(skip)] Context),
    OpenThemesFolder,
    SetDiagramBackground(#[serde(skip)] Context, state::DiagramBackground),
    ExportModal(egui::Id, String, ExportType),
    Export(egui::Id, String, ExportType, Box<egui::Visuals>),
    CopyExport(
        #[serde(skip)] Context,
        egui::Id,
        ExportType,
        Box<egui::Visuals>,
    ),
    FontSizeModal(egui::Id),
    SaveEditorToLibraryRequest(#[serde(skip)] Context, egui::Id),
    SaveEditorToLibrary {
        editor_id: egui::Id,
        path: String,
        overwrite: bool,
    },
    ExportEditorLibraryEntry(egui::Id),
    RequestRename(#[serde(skip)] Context, egui::Id),
    RenameWindow(egui::Id, String),
    RequestRedraw(#[serde(skip)] Context, egui::Id),
    UpdateRender(#[serde(skip)] Context, egui::Id, String),
    UpdateProlog(#[serde(skip)] Context, egui::Id, String),
    UpdateTcl(#[serde(skip)] Context, egui::Id, String),
    UpdateClips(#[serde(skip)] Context, egui::Id, String),
    UpdateMruby(#[serde(skip)] Context, egui::Id, String),
    UpdateHagoromo(#[serde(skip)] Context, egui::Id, String),
    UpdatePlainText(#[serde(skip)] Context, egui::Id),
    ResetError(egui::Id),
    UpdateGeneratedContent(egui::Id, String),
    SetRenderEnabled(#[serde(skip)] Context, egui::Id, bool),
    SetOutputType(#[serde(skip)] Context, egui::Id, OutputType),
    SetSvgbobEditMode(egui::Id, SvgbobEditMode),
    DeleteWindow(egui::Id),
    ToggleWindow(Window),
    ToggleWindowById(egui::Id),
    NewWindow(#[serde(skip)] Context, WindowType),
    RecreateSvg(#[serde(skip)] Context, egui::Id),
    ReloadSvgs(#[serde(skip)] Context),
    Refresh(#[serde(skip)] Context, egui::Id),
    RefreshWorkspace(#[serde(skip)] Context),
    ResetWorkspaceRequest,
    ResetWorkspace,
    SaveWorkspace,
    LoadWorkspaceRequest,
    LoadWorkspace(String),
    OpenLibraryEntry(#[serde(skip)] Context, String),
    DeleteLibraryEntryRequest(String),
    DeleteLibraryEntry(String),
    ExportLibraryEntry(String),
    ImportLibraryEntries,
    ImportLibraryEntry(state::LibraryEntry, bool),
    SwitchWorkspace(state::WorkspaceId),
    NewWorkspaceRequest,
    NewWorkspace(String),
    RenameWorkspaceRequest(state::WorkspaceId),
    RenameWorkspace(state::WorkspaceId, String),
    DuplicateWorkspace(state::WorkspaceId),
    DeleteWorkspaceRequest(state::WorkspaceId),
}

#[derive(Default, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum SvgbobEditMode {
    #[default]
    Insert,
    Replace,
}

impl SvgbobEditMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Insert => "Insert",
            Self::Replace => "Replace",
        }
    }

    pub const fn toggled(self) -> Self {
        match self {
            Self::Insert => Self::Replace,
            Self::Replace => Self::Insert,
        }
    }
}

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum EditorType {
    Prolog,
    Pikchr,
    Svgbob,
    Tcl,
    Clips,
    Mruby,
    Hagoromo,
    PlainText,
}

#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone, Copy, Default)]
pub enum OutputType {
    #[default]
    Pikchr,
    Svgbob,
    /// Plain text, rendered as is. CLIPS `(include NAME)` takes it in place
    /// of the editor's source.
    Text,
}

impl OutputType {
    /// The output types every diagram editor offers.
    pub const DIAGRAM: &[Self] = &[Self::Pikchr, Self::Svgbob];

    /// The output type the toolbar toggle switches to after `self`, cycling
    /// through `offered`.
    pub fn next_in(self, offered: &[Self]) -> Self {
        let at = offered.iter().position(|&kind| kind == self);
        at.and_then(|at| offered.get(at + 1))
            .or_else(|| offered.first())
            .copied()
            .unwrap_or(self)
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Pikchr => "Pikchr",
            Self::Svgbob => "Svgbob",
            Self::Text => "Text",
        }
    }

    pub const fn source_extension(self) -> &'static str {
        match self {
            Self::Pikchr => "pikchr",
            Self::Svgbob | Self::Text => "txt",
        }
    }
}

/// The language a Render window exports as source. Every [`OutputType`] is
/// one; editors without an Output Type (Hagoromo) add their own.
#[derive(PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize, Clone, Copy, Default)]
pub enum SourceFormat {
    #[default]
    Pikchr,
    Svgbob,
    Hagoromo,
    Text,
}

impl SourceFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Pikchr => OutputType::Pikchr.label(),
            Self::Svgbob => OutputType::Svgbob.label(),
            Self::Hagoromo => "Hagoromo",
            Self::Text => OutputType::Text.label(),
        }
    }

    pub const fn source_extension(self) -> &'static str {
        match self {
            Self::Pikchr => OutputType::Pikchr.source_extension(),
            Self::Svgbob => OutputType::Svgbob.source_extension(),
            Self::Hagoromo => "glu",
            Self::Text => OutputType::Text.source_extension(),
        }
    }
}

impl From<OutputType> for SourceFormat {
    fn from(output_type: OutputType) -> Self {
        match output_type {
            OutputType::Pikchr => Self::Pikchr,
            OutputType::Svgbob => Self::Svgbob,
            OutputType::Text => Self::Text,
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub enum Window {
    Logger,
    Debugger,
}

#[cfg(test)]
mod tests {
    use super::OutputType;

    #[test]
    fn output_toggle_cycles_through_the_offered_types() {
        let ruby = [OutputType::Pikchr, OutputType::Svgbob, OutputType::Text];
        assert_eq!(OutputType::Svgbob.next_in(&ruby), OutputType::Text);
        assert_eq!(OutputType::Text.next_in(&ruby), OutputType::Pikchr);
        assert_eq!(OutputType::Svgbob.next_in(OutputType::DIAGRAM), OutputType::Pikchr);
        // A type the editor no longer offers switches to the first offered.
        assert_eq!(OutputType::Text.next_in(OutputType::DIAGRAM), OutputType::Pikchr);
    }
}
