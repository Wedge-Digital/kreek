use askama::Template;

/// Rend le widget d'upload Cloudinary de kreek hors de tout template appelant.
///
/// Askama résout `import` statiquement, comme `extends` : un BC extractible qui
/// appelle la macro directement emporte le dossier de templates du host — et,
/// ici, le compte Cloudinary de kreek codé en dur dans la macro. Le host rend
/// donc le fragment lui-même et le passe en `String` au BC.
#[derive(Template)]
#[template(path = "upload-widget-fragment.html")]
struct UploadWidgetFragment<'a> {
    field_id: &'a str,
    initial_value: &'a str,
    folder: &'a str,
    label: &'a str,
    error: Option<&'a str>,
}

pub fn render_upload_widget(
    field_id: &str,
    initial_value: &str,
    folder: &str,
    label: &str,
    error: Option<&str>,
) -> String {
    let fragment = UploadWidgetFragment {
        field_id,
        initial_value,
        folder,
        label,
        error,
    };
    fragment.render().unwrap_or_else(|e| {
        tracing::error!("rendu du widget d'upload échoué: {e}");
        String::new()
    })
}
