pub(crate) fn transform(url: &str, transform: &str) -> String {
    const MARKER: &str = "/upload/";
    if let Some(pos) = url.find(MARKER) {
        let (before, after) = url.split_at(pos + MARKER.len());
        format!("{}{}/{}", before, transform, after)
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_transform_after_upload() {
        let url = "https://res.cloudinary.com/demo/image/upload/sample.jpg";
        let result = transform(url, "c_fill,w_100,h_100");
        assert_eq!(
            result,
            "https://res.cloudinary.com/demo/image/upload/c_fill,w_100,h_100/sample.jpg"
        );
    }

    #[test]
    fn returns_url_unchanged_when_no_upload_marker() {
        let url = "https://example.com/image.jpg";
        let result = transform(url, "c_fill,w_100,h_100");
        assert_eq!(result, url);
    }
}
