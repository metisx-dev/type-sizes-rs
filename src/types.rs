#[derive(Debug, PartialEq)]
pub struct Field {
    pub name: String,
    pub size: u64,
    pub alignment: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub struct ClosureVar {
    pub name: String,
    pub size: u64,
    pub offset: Option<u64>,
    pub alignment: Option<u64>,
    pub type_info: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum StructEntry {
    Field(Field),
    Upvar(ClosureVar),
    Local(ClosureVar),
    Padding { size: u64 },
}

#[derive(Debug, PartialEq)]
pub struct Variant {
    pub name: String,
    pub size: u64,
    pub entries: Vec<StructEntry>,
}

#[derive(Debug, PartialEq)]
pub enum TypeKind {
    Struct {
        entries: Vec<StructEntry>,
    },
    Enum {
        discriminant_size: u64,
        variants: Vec<Variant>,
    },
    Union {
        fields: Vec<Field>,
    },
}

#[derive(Debug, PartialEq)]
pub struct TypeLayout {
    pub name: String,
    pub size: u64,
    pub alignment: u64,
    pub kind: TypeKind,
    pub unhandled_lines: Vec<String>,
    pub raw_lines: Vec<String>,
}

#[derive(Debug)]
pub enum VerificationError {
    StructSizeMismatch {
        expected: u64,
        actual: u64,
    },
    VariantSizeMismatch {
        variant_name: String,
        expected: u64,
        actual: u64,
    },
    // NEW: Union 검증 오류 추가
    UnionSizeMismatch {
        expected: u64,
        actual_max: u64,
    },
    EnumTotalSizeMismatch {
        expected: u64,
        calculated_min: u64,
    },
}

impl StructEntry {
    fn size(&self) -> u64 {
        match self {
            StructEntry::Field(f) => f.size,
            StructEntry::Upvar(c) => c.size,
            StructEntry::Local(c) => c.size,
            StructEntry::Padding { size } => *size,
        }
    }
}

impl TypeLayout {
    pub fn verify(&self) -> Result<(), VerificationError> {
        match &self.kind {
            TypeKind::Struct { entries } => {
                let calculated_size: u64 = entries.iter().map(|e| e.size()).sum();
                if self.size == calculated_size {
                    Ok(())
                } else {
                    Err(VerificationError::StructSizeMismatch {
                        expected: self.size,
                        actual: calculated_size,
                    })
                }
            }
            TypeKind::Enum {
                variants,
                discriminant_size,
            } => {
                for variant in variants {
                    let calculated_variant_size: u64 =
                        variant.entries.iter().map(|e| e.size()).sum();
                    let expected_size = variant.size;
                    if calculated_variant_size == expected_size
                        || (*discriminant_size > 0
                            && calculated_variant_size == expected_size + *discriminant_size)
                    {
                        continue;
                    }
                    return Err(VerificationError::VariantSizeMismatch {
                        variant_name: variant.name.clone(),
                        expected: expected_size,
                        actual: calculated_variant_size,
                    });
                }
                let max_variant_size = variants.iter().map(|v| v.size).max().unwrap_or(0);
                let min_required_additive = discriminant_size + max_variant_size;
                let min_required_niche = (*discriminant_size).max(max_variant_size);
                if self.size >= min_required_additive || self.size == min_required_niche {
                    Ok(())
                } else {
                    Err(VerificationError::EnumTotalSizeMismatch {
                        expected: self.size,
                        calculated_min: min_required_additive,
                    })
                }
            }
            TypeKind::Union { fields } => {
                let max_field_size = fields.iter().map(|f| f.size).max().unwrap_or(0);
                if self.size == max_field_size {
                    Ok(())
                } else {
                    Err(VerificationError::UnionSizeMismatch {
                        expected: self.size,
                        actual_max: max_field_size,
                    })
                }
            }
        }
    }
}
