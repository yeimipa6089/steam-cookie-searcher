pub fn xml_tag(xml: &str, tag: &str) -> String {
    let start_tag = format!("<{}>", tag);
    let end_tag = format!("</{}>", tag);
    if let Some(start_idx) = xml.find(&start_tag) {
        let content_start = start_idx + start_tag.len();
        if let Some(end_idx) = xml[content_start..].find(&end_tag) {
            let mut content = &xml[content_start..content_start + end_idx];
            let cdata_start = "<![CDATA[";
            let cdata_end = "]]>";
            if content.starts_with(cdata_start) && content.ends_with(cdata_end) {
                content = &content[cdata_start.len()..content.len() - cdata_end.len()];
            }
            return content.trim().to_string();
        }
    }
    String::new()
}
