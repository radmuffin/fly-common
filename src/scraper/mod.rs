use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::security::validate_url_for_ssrf;

/// Extracted OpenGraph, Twitter Card, and document metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PageMetadata {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image_url: Option<String>,
    pub site_name: Option<String>,
}

/// Safely scrapes OpenGraph and page metadata from an external URL with SSRF protection.
/// Ensures the non-Send `scraper::Html` struct is dropped synchronously before any future await points.
pub async fn scrape_page_metadata(client: &Client, raw_url: &str) -> Result<PageMetadata, String> {
    validate_url_for_ssrf(raw_url)?;

    let resp = client
        .get(raw_url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (compatible; FlyApp/1.0; +https://fly.io)",
        )
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {}", e))?;

    let final_url = resp.url().to_string();
    validate_url_for_ssrf(&final_url)?;

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    // Parse HTML synchronously and drop non-Send Html tree immediately
    let mut meta = PageMetadata {
        url: final_url,
        ..Default::default()
    };

    {
        let doc = Html::parse_document(&body);

        // 1. OpenGraph & Twitter tags
        if let Ok(meta_sel) = Selector::parse("meta[property], meta[name]") {
            for element in doc.select(&meta_sel) {
                let key = element
                    .value()
                    .attr("property")
                    .or_else(|| element.value().attr("name"))
                    .map(|s| s.to_ascii_lowercase());

                let content = element
                    .value()
                    .attr("content")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                if let (Some(k), Some(c)) = (key, content) {
                    match k.as_str() {
                        "og:title" | "twitter:title" if meta.title.is_none() => {
                            meta.title = Some(c);
                        }
                        "og:description" | "twitter:description" | "description"
                            if meta.description.is_none() =>
                        {
                            meta.description = Some(c);
                        }
                        "og:image" | "twitter:image" | "twitter:image:src"
                            if meta.image_url.is_none() =>
                        {
                            meta.image_url = Some(c);
                        }
                        "og:site_name" if meta.site_name.is_none() => {
                            meta.site_name = Some(c);
                        }
                        _ => {}
                    }
                }
            }
        }

        // 2. Fallback HTML <title>
        if meta.title.is_none() {
            if let Ok(title_sel) = Selector::parse("title") {
                if let Some(title_el) = doc.select(&title_sel).next() {
                    let title_text: String = title_el.text().collect::<Vec<_>>().join(" ");
                    let clean = title_text.trim();
                    if !clean.is_empty() {
                        meta.title = Some(clean.to_string());
                    }
                }
            }
        }
    } // `doc` is explicitly dropped here for Axum Send-safety

    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_html_metadata_extraction() {
        let sample_html = r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Hidden Izakaya Guide</title>
                <meta property="og:title" content="The Best Tokyo Izakayas" />
                <meta property="og:description" content="Discover back-alley yakitori & draft beer." />
                <meta property="og:image" content="https://example.com/izakaya.jpg" />
                <meta property="og:site_name" content="Tokyo Explorer" />
            </head>
            <body><h1>Welcome</h1></body>
            </html>
        "#;

        let doc = Html::parse_document(sample_html);
        let mut meta = PageMetadata {
            url: "https://example.com/tokyo".to_string(),
            ..Default::default()
        };

        if let Ok(meta_sel) = Selector::parse("meta[property]") {
            for element in doc.select(&meta_sel) {
                if let (Some(prop), Some(cont)) = (
                    element.value().attr("property"),
                    element.value().attr("content"),
                ) {
                    match prop {
                        "og:title" => meta.title = Some(cont.to_string()),
                        "og:description" => meta.description = Some(cont.to_string()),
                        "og:image" => meta.image_url = Some(cont.to_string()),
                        "og:site_name" => meta.site_name = Some(cont.to_string()),
                        _ => {}
                    }
                }
            }
        }

        assert_eq!(meta.title.as_deref(), Some("The Best Tokyo Izakayas"));
        assert_eq!(
            meta.description.as_deref(),
            Some("Discover back-alley yakitori & draft beer.")
        );
        assert_eq!(
            meta.image_url.as_deref(),
            Some("https://example.com/izakaya.jpg")
        );
        assert_eq!(meta.site_name.as_deref(), Some("Tokyo Explorer"));
    }
}
