use super::{Context, Field};
use protocol::Failure;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Condition {
    pub field: Field,
    minimum: Option<u64>,
}

impl Condition {
    pub fn parse(text: &str) -> Result<Self, Failure> {
        let Some((field, minimum)) = text.split_once(" >= ") else {
            return Ok(Self {
                field: Field::parse(text)?,
                minimum: None,
            });
        };
        let field = Field::parse(field.trim())?;
        if !field.numeric() {
            return Err(Failure::Payload);
        }
        let minimum = minimum.trim().parse().map_err(|_| Failure::Payload)?;
        Ok(Self {
            field,
            minimum: Some(minimum),
        })
    }

    pub fn test<C: Context>(&self, context: &C) -> bool {
        let value = context.field(self.field);
        match self.minimum {
            None => !value.is_empty(),
            Some(minimum) => value.parse::<u64>().is_ok_and(|value| value >= minimum),
        }
    }
}
