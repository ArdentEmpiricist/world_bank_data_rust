//! Text measurement, truncation, and wrapping utilities.

/// Heuristic: estimate pixel width of text (Plotters has no built-in text measuring).
pub fn estimate_text_width_px(text: &str, font_px: u32) -> u32 {
    ((text.chars().count() as f32) * (font_px as f32) * 0.60).ceil() as u32
}

/// Truncate to fit `max_px` and add a single ellipsis if needed.
pub fn truncate_to_width(text: &str, font_px: u32, max_px: u32) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        let next = format!("{out}{ch}");
        if estimate_text_width_px(&next, font_px) > max_px {
            if !out.is_empty() {
                if estimate_text_width_px(&(out.clone() + "…"), font_px) <= max_px {
                    out.push('…');
                } else if out.len() > 1 {
                    out.pop();
                    out.push('…');
                }
            }
            return out;
        }
        out = next;
    }
    out
}

/// Wrap text to fit within a maximum pixel width, breaking on word boundaries where possible.
pub fn wrap_text_to_width(text: &str, font_px: u32, max_px: u32) -> Vec<String> {
    if max_px <= 12 {
        return vec![truncate_to_width(text, font_px, max_px)];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let candidate = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if estimate_text_width_px(&candidate, font_px) <= max_px {
            cur = candidate;
        } else if cur.is_empty() {
            // Single long word: hard-break by characters
            let mut buf = String::new();
            for ch in word.chars() {
                let cand = format!("{buf}{ch}");
                if estimate_text_width_px(&cand, font_px) > max_px {
                    if buf.is_empty() {
                        lines.push(truncate_to_width(word, font_px, max_px));
                        buf.clear();
                        break;
                    } else {
                        lines.push(buf);
                        buf = ch.to_string();
                    }
                } else {
                    buf = cand;
                }
            }
            if !buf.is_empty() {
                lines.push(buf);
            }
        } else {
            lines.push(cur);
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}
