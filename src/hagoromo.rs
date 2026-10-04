//! Hagoromo output: a Gluon script evaluates to a diagram, rendered to SVG.
//!
//! One Gluon VM is created on first use and kept for the process lifetime,
//! since building it (and compiling the Hagoromo prelude) costs far more than
//! evaluating a script. Evaluations are serialized through a mutex.

use std::sync::{Mutex, OnceLock};

use gluon::{RootedThread, ThreadExt, vm::api::UserdataValue};
use hagoromo::gluon::userdata::GDiagram;

/// Module name every script is compiled under. Reusing one name keeps the
/// VM's compilation database from growing with each render.
const SCRIPT_MODULE: &str = "diagramide_script";

static VM: OnceLock<Result<Mutex<RootedThread>, String>> = OnceLock::new();

fn vm() -> Result<&'static Mutex<RootedThread>, String> {
    VM.get_or_init(|| {
        let vm = gluon::new_vm();
        // The Gluon std prelude fails to register on current Rust toolchains
        // (gluon 0.18.2). Hagoromo scripts need only its own prelude.
        vm.get_database_mut().set_implicit_prelude(false);
        hagoromo::gluon::register(&vm)
            .map_err(|error| format!("Failed to register Hagoromo bindings: {error}"))?;
        vm.load_script("hagoromo", hagoromo::gluon::PRELUDE)
            .map_err(|error| format!("Failed to load Hagoromo prelude: {error}"))?;
        Ok(Mutex::new(vm))
    })
    .as_ref()
    .map_err(Clone::clone)
}

/// A name a script can reference, with its Gluon type.
#[derive(Debug, Clone, PartialEq)]
pub struct Completion {
    pub name: String,
    pub signature: String,
}

static CATALOG: OnceLock<Result<Vec<Completion>, String>> = OnceLock::new();
static CATALOG_WARMUP: std::sync::Once = std::sync::Once::new();

/// Names exported by the Hagoromo prelude and `hagoromo.prim`, nested records
/// flattened to dotted paths (`color.red`).
///
/// Returns an empty slice until the VM is ready; the first call starts
/// building it on a background thread so the UI never waits for the prelude.
pub fn completions() -> &'static [Completion] {
    if let Some(Ok(catalog)) = CATALOG.get() {
        return catalog;
    }
    CATALOG_WARMUP.call_once(|| {
        std::thread::spawn(|| {
            let _ = completions_blocking();
        });
    });
    &[]
}

fn completions_blocking() -> Result<&'static [Completion], String> {
    CATALOG
        .get_or_init(|| {
            let vm = vm()?
                .lock()
                .map_err(|_| "Hagoromo VM lock was poisoned".to_string())?;
            let mut catalog = Vec::new();
            for module in ["hagoromo", "hagoromo.prim"] {
                let typ = vm
                    .get_global_type(module)
                    .map_err(|error| error.to_string())?;
                collect_fields("", &typ, &mut catalog);
            }
            catalog.sort_by(|a, b| a.name.cmp(&b.name));
            catalog.dedup_by(|a, b| a.name == b.name);
            Ok(catalog)
        })
        .as_ref()
        .map(Vec::as_slice)
        .map_err(Clone::clone)
}

fn collect_fields(prefix: &str, typ: &gluon::base::types::ArcType, out: &mut Vec<Completion>) {
    use gluon::base::types::{Type, remove_forall, row_iter};

    for field in row_iter(typ) {
        let name = field.name.declared_name();
        // Operators like `<>` are used infix, never typed as identifiers.
        if !name.starts_with(|c: char| c.is_alphabetic() || c == '_') {
            continue;
        }
        let path = format!("{prefix}{name}");
        if let Type::Record(_) = **remove_forall(&field.typ) {
            out.push(Completion {
                name: path.clone(),
                signature: "record".to_string(),
            });
            collect_fields(&format!("{path}."), &field.typ, out);
        } else {
            out.push(Completion {
                name: path,
                signature: short_signature(&field.typ.to_string()),
            });
        }
    }
}

/// One line, without the module paths of Hagoromo's and Gluon's types.
fn short_signature(signature: &str) -> String {
    signature
        .replace("hagoromo.types.", "")
        .replace("std.types.", "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Evaluate `source` as a Gluon expression producing a Hagoromo diagram and
/// render it to SVG.
pub fn render_hagoromo(source: &str) -> Result<String, String> {
    if source.trim().is_empty() {
        return Err("Script is empty.".to_string());
    }
    let vm = vm()?
        .lock()
        .map_err(|_| "Hagoromo VM lock was poisoned".to_string())?;
    let (UserdataValue(diagram), _) = vm
        .run_expr::<UserdataValue<GDiagram>>(SCRIPT_MODULE, source)
        .map_err(|error| error.emit_string().unwrap_or_else(|_| error.to_string()))?;
    Ok(hagoromo::render_svg(
        &diagram.0,
        &hagoromo::RenderOptions::default(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_renders_to_svg() {
        let svg = render_hagoromo(
            r#"
            let { prim, (|||), (|>) } = import! hagoromo
            let { circle, square, fc, color } = prim
            circle 1.0 ||| (square 2.0 |> fc color.red)
            "#,
        )
        .unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<circle"));
        assert!(svg.contains("#ff0000"));
    }

    #[test]
    fn errors_are_reported_with_location() {
        let error = render_hagoromo("let x = 1\nx |> nonsense").unwrap_err();
        assert!(error.contains("nonsense"), "{error}");
    }

    #[test]
    fn non_diagram_results_are_rejected() {
        let error = render_hagoromo("42").unwrap_err();
        assert!(!error.is_empty());
    }

    #[test]
    fn evaluations_use_fresh_scopes() {
        let define = r#"
            let { prim } = import! hagoromo
            let leaked = prim.circle 1.0
            leaked
        "#;
        render_hagoromo(define).unwrap();
        let error = render_hagoromo("leaked").expect_err("bindings must not persist");
        assert!(error.contains("leaked"), "{error}");
        render_hagoromo(define).unwrap();
    }

    #[test]
    fn output_rasterizes_with_filled_shapes() {
        let svg = crate::render::inject_svg_style(&render_hagoromo("let { prim, (|>) } = import! hagoromo\nprim.square 4.0 |> prim.fc prim.color.black").unwrap());
        let image = crate::image::render_svg_to_image(
            &svg,
            1.0,
            crate::image::RenderBackground::Transparent,
        )
        .expect("hagoromo SVG must be accepted by the rasterizer");
        let [width, height] = image.size;
        let center = image.pixels[(height / 2) * width + width / 2];
        assert_eq!(center.a(), 255, "square fill should cover the center");
        assert_eq!(image.pixels[0].a(), 0, "padding should stay transparent");
    }

    #[test]
    fn catalog_lists_prim_functions_with_types() {
        let catalog = completions_blocking().unwrap();
        let circle = catalog.iter().find(|c| c.name == "circle").unwrap();
        assert!(circle.signature.ends_with("-> Diagram"), "{circle:?}");
        assert!(!circle.signature.contains('\n'));
    }

    #[test]
    fn catalog_flattens_nested_records() {
        let catalog = completions_blocking().unwrap();
        let names: Vec<&str> = catalog.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"prim"));
        assert!(names.contains(&"color"));
        assert!(names.contains(&"color.red"));
        assert!(
            names.iter().all(|name| !name.contains('<')),
            "operators are excluded"
        );
    }

    #[test]
    fn empty_script_is_an_error() {
        assert!(render_hagoromo("   \n").is_err());
    }
}
