#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenPool {
    pub id: String,
    pub label: String,
    pub token: String,
    pub background: Option<String>,
    pub count: u32,
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub active: bool,
}

impl TokenPool {
    pub fn new(id: impl Into<String>, label: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            token: token.into(),
            background: None,
            count: 0,
            min: None,
            max: None,
            active: false,
        }
    }

    pub fn configured(
        id: impl Into<String>,
        label: impl Into<String>,
        token: impl Into<String>,
        background: Option<String>,
        count: u32,
        min: Option<u32>,
        max: Option<u32>,
        active: bool,
    ) -> Result<Self, String> {
        let pool = Self {
            id: id.into(),
            label: label.into(),
            token: token.into(),
            background,
            count,
            min,
            max,
            active,
        };
        pool.validate()?;
        Ok(pool)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("Token pool id cannot be empty".to_string());
        }
        if self.label.trim().is_empty() {
            return Err(format!("Token pool '{}' must have a label", self.id));
        }
        if self.token.trim().is_empty() {
            return Err(format!("Token pool '{}' must have a token", self.id));
        }
        if let (Some(min), Some(max)) = (self.min, self.max) {
            if min > max {
                return Err(format!(
                    "Token pool '{}' has min {min} greater than max {max}",
                    self.id
                ));
            }
        }
        if let Some(min) = self.min {
            if self.count < min {
                return Err(format!(
                    "Token pool '{}' starts below its min of {min}",
                    self.id
                ));
            }
        }
        if let Some(max) = self.max {
            if self.count > max {
                return Err(format!(
                    "Token pool '{}' starts above its max of {max}",
                    self.id
                ));
            }
        }
        Ok(())
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn background(&self) -> Option<&str> {
        self.background.as_deref()
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    pub fn add_tokens(&mut self, amount: u32) -> Result<(), String> {
        let next = self
            .count
            .checked_add(amount)
            .ok_or_else(|| format!("Token pool '{}' overflowed", self.id))?;
        if let Some(max) = self.max {
            if next > max {
                return Err(format!(
                    "Token pool '{}' cannot exceed max of {max}",
                    self.id
                ));
            }
        }
        self.count = next;
        Ok(())
    }

    pub fn remove_tokens(&mut self, amount: u32) -> Result<(), String> {
        let next = self
            .count
            .checked_sub(amount)
            .ok_or_else(|| format!("Token pool '{}' cannot go below zero", self.id))?;
        if let Some(min) = self.min {
            if next < min {
                return Err(format!(
                    "Token pool '{}' cannot go below min of {min}",
                    self.id
                ));
            }
        }
        self.count = next;
        Ok(())
    }

    pub fn parse_list(raw: &str) -> Result<Vec<Self>, String> {
        raw.split(';')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(Self::parse)
            .collect()
    }

    fn parse(raw: &str) -> Result<Self, String> {
        let mut id = None;
        let mut label = None;
        let mut token = None;
        let mut background = None;
        let mut count = 0;
        let mut min = None;
        let mut max = None;
        let mut active = false;

        for part in raw
            .split('|')
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let Some((key, value)) = part.split_once('=') else {
                return Err(format!("Invalid token pool part '{part}'"));
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "id" => id = Some(value.to_string()),
                "label" => label = Some(value.to_string()),
                "token" | "icon" => token = Some(value.to_string()),
                "background" => background = Some(value.to_string()),
                "starting" | "count" => {
                    count = value
                        .parse()
                        .map_err(|_| format!("Invalid token pool count '{value}'"))?
                }
                "min" => {
                    min = Some(
                        value
                            .parse()
                            .map_err(|_| format!("Invalid token pool min '{value}'"))?,
                    )
                }
                "max" => {
                    max = Some(
                        value
                            .parse()
                            .map_err(|_| format!("Invalid token pool max '{value}'"))?,
                    )
                }
                "active" => active = parse_bool(value),
                _ => return Err(format!("Unknown token pool field '{key}'")),
            }
        }

        let id = id.ok_or_else(|| "Token pool is missing required id".to_string())?;
        let label = label.unwrap_or_else(|| id.clone());
        let token = token.unwrap_or_else(|| label.clone());
        let pool = Self {
            id,
            label,
            token,
            background,
            count,
            min,
            max,
            active,
        };
        pool.validate()?;
        Ok(pool)
    }
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "y"
    )
}

#[cfg(test)]
mod tests {
    use super::TokenPool;

    #[test]
    fn parses_multiple_token_pools() {
        let pools = TokenPool::parse_list(
            "id=charge|label=Charge|token=fa-bolt|background=amber|starting=1|min=0|max=3|active=true;\
             id=shield|token=fa-shield",
        )
        .expect("parse token pools");

        assert_eq!(pools.len(), 2);
        assert_eq!(pools[0].token(), "fa-bolt");
        assert_eq!(pools[0].background(), Some("amber"));
        assert!(pools[0].active);
        assert_eq!(pools[1].label, "shield");
    }

    #[test]
    fn enforces_token_pool_bounds() {
        let err = TokenPool::parse_list("id=charge|token=fa-bolt|starting=5|max=3")
            .expect_err("invalid token pool");
        assert!(err.contains("max"));
    }

    #[test]
    fn enforces_minimum_when_removing_tokens() {
        let mut pool = TokenPool::parse_list("id=charge|token=fa-bolt|starting=2|min=1")
            .expect("parse token pool")
            .remove(0);
        assert!(pool.remove_tokens(2).is_err());
        assert!(pool.remove_tokens(1).is_ok());
        assert_eq!(pool.count, 1);
    }
}
