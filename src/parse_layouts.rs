use crate::types::{ClosureVar, Field, StructEntry, TypeKind, TypeLayout, Variant};
use lazy_static::lazy_static;
use regex::Regex;
use std::io::{self, BufRead};

lazy_static! {
    static ref RE_TYPE: Regex = Regex::new(r"^\s*print-type-size\s+type: `(.+?)`: (\d+) bytes, alignment: (\d+) bytes").unwrap();
    static ref RE_FIELD: Regex = Regex::new(r"^\s*print-type-size\s+field (?:`(\..+?)`|(\..+?)): (\d+) bytes(.*)").unwrap();
    static ref RE_PADDING: Regex = Regex::new(r"^\s*print-type-size\s+padding: (\d+) bytes").unwrap();
    static ref RE_END_PADDING: Regex = Regex::new(r"^\s*print-type-size\s+end padding: (\d+) bytes").unwrap();
    static ref RE_VARIANT: Regex = Regex::new(r"^\s*print-type-size\s+variant `(.+?)`: (\d+) bytes").unwrap();
    static ref RE_DISCRIMINANT: Regex = Regex::new(r"^\s*print-type-size\s+discriminant: (\d+) bytes").unwrap();
    static ref RE_UPVAR: Regex = Regex::new(r"^\s*print-type-size\s+upvar `(.+?)`: (\d+) bytes(?:, offset: (\d+) bytes, alignment: (\d+) bytes)?").unwrap();
    static ref RE_LOCAL: Regex = Regex::new(r"^\s*print-type-size\s+local `(.+?)`: (\d+) bytes(?:, type: (.+))?").unwrap();
    static ref RE_ATTR_OFFSET: Regex = Regex::new(r"offset: (\d+)").unwrap();
    static ref RE_ATTR_ALIGN: Regex = Regex::new(r"alignment: (\d+)").unwrap();
}

pub fn parse_layouts(reader: impl BufRead) -> io::Result<Vec<TypeLayout>> {
    let mut layouts = Vec::new();
    let mut current_layout: Option<TypeLayout> = None;
    let mut current_variant: Option<Variant> = None;

    let finalize_layout = |layout: &mut TypeLayout, current_variant: &mut Option<Variant>| {
        if let Some(variant) = current_variant.take() {
            if let TypeKind::Enum { variants, .. } = &mut layout.kind {
                variants.push(variant);
            }
        }
        if let TypeKind::Enum { variants, .. } = &mut layout.kind {
            if variants.len() == 1 && variants[0].name == layout.name {
                let union_variant = variants.remove(0);
                let fields = union_variant
                    .entries
                    .into_iter()
                    .filter_map(|e| match e {
                        StructEntry::Field(f) => Some(f),
                        _ => None,
                    })
                    .collect();
                layout.kind = TypeKind::Union { fields };
            }
        }
    };

    for line_result in reader.lines() {
        let original_line = line_result?;
        if original_line.trim().is_empty() {
            continue;
        }

        if let Some(caps) = RE_TYPE.captures(&original_line) {
            if let Some(mut layout) = current_layout.take() {
                finalize_layout(&mut layout, &mut current_variant);
                layouts.push(layout);
            }
            current_layout = Some(TypeLayout {
                name: caps[1].to_string(),
                size: caps[2].parse().unwrap(),
                alignment: caps[3].parse().unwrap(),
                kind: TypeKind::Struct {
                    entries: Vec::new(),
                },
                unhandled_lines: Vec::new(),
                raw_lines: vec![original_line],
            });
            continue;
        }

        let layout = match current_layout.as_mut() {
            Some(l) => l,
            None => continue,
        };

        layout.raw_lines.push(original_line.clone());

        let mut handled = false;
        if let Some(caps) = RE_VARIANT.captures(&original_line) {
            handled = true;
            if let Some(variant) = current_variant.take() {
                if let TypeKind::Enum { variants, .. } = &mut layout.kind {
                    variants.push(variant)
                }
            }
            let name_with_ticks = caps[1].to_string();
            current_variant = Some(Variant {
                name: name_with_ticks.trim_matches('`').to_string(),
                size: caps[2].parse().unwrap(),
                entries: Vec::new(),
            });
            if let TypeKind::Struct { .. } = layout.kind {
                layout.kind = TypeKind::Enum {
                    discriminant_size: 0,
                    variants: Vec::new(),
                };
            }
        } else if let Some(caps) = RE_DISCRIMINANT.captures(&original_line) {
            handled = true;
            if let TypeKind::Enum {
                discriminant_size, ..
            } = &mut layout.kind
            {
                *discriminant_size = caps[1].parse().unwrap();
            } else {
                layout.kind = TypeKind::Enum {
                    discriminant_size: caps[1].parse().unwrap(),
                    variants: Vec::new(),
                };
            }
        } else if let Some(caps) = RE_UPVAR.captures(&original_line) {
            handled = true;
            let var = ClosureVar {
                name: caps.get(1).unwrap().as_str().to_string(),
                size: caps.get(2).unwrap().as_str().parse().unwrap(),
                offset: caps.get(3).map(|m| m.as_str().parse().unwrap()),
                alignment: caps.get(4).map(|m| m.as_str().parse().unwrap()),
                type_info: None,
            };
            if let Some(variant) = current_variant.as_mut() {
                variant.entries.push(StructEntry::Upvar(var));
            }
        } else if let Some(caps) = RE_LOCAL.captures(&original_line) {
            handled = true;
            let var = ClosureVar {
                name: caps.get(1).unwrap().as_str().to_string(),
                size: caps.get(2).unwrap().as_str().parse().unwrap(),
                offset: None,
                alignment: None,
                type_info: caps.get(3).map(|m| m.as_str().to_string()),
            };
            if let Some(variant) = current_variant.as_mut() {
                variant.entries.push(StructEntry::Local(var));
            }
        } else if let Some(caps) = RE_FIELD.captures(&original_line) {
            handled = true;
            // 이름과 크기 파싱
            let name = caps
                .get(1)
                .or_else(|| caps.get(2))
                .unwrap()
                .as_str()
                .to_string();
            let size = caps[3].parse().unwrap();
            let attributes_str = caps.get(4).unwrap().as_str();

            // MODIFIED: 나머지 문자열에서 offset과 alignment를 추가로 파싱
            let offset = RE_ATTR_OFFSET
                .captures(attributes_str)
                .map(|c| c[1].parse().unwrap());
            let alignment = RE_ATTR_ALIGN
                .captures(attributes_str)
                .map(|c| c[1].parse().unwrap());

            let field = Field {
                name,
                size,
                alignment,
                offset,
            };
            let entry = StructEntry::Field(field);

            if let Some(variant) = current_variant.as_mut() {
                variant.entries.push(entry);
            } else if let TypeKind::Struct { entries } = &mut layout.kind {
                entries.push(entry);
            }
        } else if let Some(caps) = RE_PADDING.captures(&original_line) {
            handled = true;
            let entry = StructEntry::Padding {
                size: caps[1].parse().unwrap(),
            };
            if let Some(variant) = current_variant.as_mut() {
                variant.entries.push(entry);
            } else if let TypeKind::Struct { entries } = &mut layout.kind {
                entries.push(entry);
            }
        } else if let Some(caps) = RE_END_PADDING.captures(&original_line) {
            handled = true;
            let entry = StructEntry::Padding {
                size: caps[1].parse().unwrap(),
            };
            if let TypeKind::Struct { entries } = &mut layout.kind {
                entries.push(entry);
            }
        }

        if !handled {
            if original_line.contains("print-type-size") {
                layout.unhandled_lines.push(original_line);
            }
        }
    }

    if let Some(mut layout) = current_layout.take() {
        finalize_layout(&mut layout, &mut current_variant);
        layouts.push(layout);
    }

    Ok(layouts)
}
