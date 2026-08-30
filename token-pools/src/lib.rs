use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenType {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub description: Option<String>,
    pub background: Option<String>,
    pub icon: String,
    pub icon_color: Option<String>,
}

/// A token can optionally represent, or be created by, a card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub id: String,
    pub game_id: String,
    pub token_type_id: String,
    pub card_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TokenPoolOwner {
    Player,
    Card,
    Creature,
    Zone,
    Battlefield,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenPoolDefinition {
    pub pool: TokenPool,
    pub owner: TokenPoolOwner,
    pub owner_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenPool {
    pub id: String,
    pub label: String,
    pub token_type_id: Option<String>,
    /// Complete Font Awesome class string.
    pub token: String,
    pub icon_color: Option<String>,
    pub background: Option<String>,
    pub starting: u32,
    pub count: u32,
    pub plus: u32,
    pub minus: u32,
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub parent_id: Option<String>,
    pub active: bool,
}

impl TokenPool {
    pub fn new(id: impl Into<String>, label: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            token_type_id: None,
            token: token.into(),
            icon_color: None,
            background: None,
            starting: 0,
            count: 0,
            plus: 1,
            minus: 1,
            min: None,
            max: None,
            parent_id: None,
            active: false,
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        let mut pool = Self::new(id, label, token);
        pool.background = background;
        pool.starting = count;
        pool.count = count;
        pool.min = min;
        pool.max = max;
        pool.active = active;
        pool.validate()?;
        Ok(pool)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("Token pool id cannot be empty".into());
        }
        if self.label.trim().is_empty() {
            return Err(format!("Token pool '{}' must have a label", self.id));
        }
        if self.token.trim().is_empty() {
            return Err(format!(
                "Token pool '{}' must have a Font Awesome icon",
                self.id
            ));
        }
        if self.plus == 0 || self.minus == 0 {
            return Err(format!(
                "Token pool '{}' plus and minus must be greater than zero",
                self.id
            ));
        }
        if self.parent_id.as_deref() == Some(self.id.as_str()) {
            return Err(format!("Token pool '{}' cannot be its own parent", self.id));
        }
        if let (Some(min), Some(max)) = (self.min, self.max) {
            if min > max {
                return Err(format!("Token pool '{}' has min greater than max", self.id));
            }
        }
        if self.min.is_some_and(|min| self.count < min) {
            return Err(format!("Token pool '{}' starts below its min", self.id));
        }
        if self.max.is_some_and(|max| self.count > max) {
            return Err(format!("Token pool '{}' starts above its max", self.id));
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
        if self.max.is_some_and(|max| next > max) {
            return Err(format!("Token pool '{}' cannot exceed max", self.id));
        }
        self.count = next;
        Ok(())
    }
    pub fn remove_tokens(&mut self, amount: u32) -> Result<(), String> {
        let next = self
            .count
            .checked_sub(amount)
            .ok_or_else(|| format!("Token pool '{}' cannot go below zero", self.id))?;
        if self.min.is_some_and(|min| next < min) {
            return Err(format!("Token pool '{}' cannot go below min", self.id));
        }
        self.count = next;
        Ok(())
    }
    pub fn parse_list(raw: &str) -> Result<Vec<Self>, String> {
        raw.split(';')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(Self::parse)
            .collect()
    }
    fn parse(raw: &str) -> Result<Self, String> {
        let mut values = HashMap::new();
        for part in raw.split('|').map(str::trim).filter(|v| !v.is_empty()) {
            let (key, value) = part
                .split_once('=')
                .ok_or_else(|| format!("Invalid token pool part '{part}'"))?;
            values.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
        pool_from_values(&values)
    }
}

pub fn ingest_token_types_csv(input: &str) -> Result<Vec<TokenType>, String> {
    let mut seen = HashSet::new();
    csv_records(input)?
        .into_iter()
        .map(|(line, v)| {
            let game_id = required(&v, "game_id", line)?;
            let id = required(&v, "id", line)?;
            if !seen.insert(format!("{game_id}/{id}")) {
                return Err(format!("line {line}: duplicate token type '{id}'"));
            }
            Ok(TokenType {
                id,
                game_id,
                name: required(&v, "name", line)?,
                description: optional(&v, "description"),
                background: optional(&v, "background"),
                icon: required(&v, "icon", line)?,
                icon_color: optional(&v, "icon_color"),
            })
        })
        .collect()
}

pub fn ingest_token_pools_csv(
    input: &str,
    types: &[TokenType],
) -> Result<Vec<TokenPoolDefinition>, String> {
    let type_map: HashMap<_, _> = types
        .iter()
        .map(|t| (format!("{}/{}", t.game_id, t.id), t))
        .collect();
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for (line, mut v) in csv_records(input)? {
        let game_id = required(&v, "game_id", line)?;
        let token_type_id = required(&v, "token_type_id", line)?;
        let key = format!("{game_id}/{token_type_id}");
        let token_type = type_map
            .get(&key)
            .ok_or_else(|| format!("line {line}: unknown token type '{key}'"))?;
        v.entry("icon".into())
            .or_insert_with(|| token_type.icon.clone());
        v.entry("label".into())
            .or_insert_with(|| token_type.name.clone());
        if let Some(value) = &token_type.background {
            v.entry("background".into())
                .or_insert_with(|| value.clone());
        }
        if let Some(value) = &token_type.icon_color {
            v.entry("icon_color".into())
                .or_insert_with(|| value.clone());
        }
        let mut pool = pool_from_values(&v).map_err(|e| format!("line {line}: {e}"))?;
        pool.token_type_id = Some(token_type_id);
        let owner = match required(&v, "owner", line)?.as_str() {
            "player" => TokenPoolOwner::Player,
            "card" => TokenPoolOwner::Card,
            "creature" => TokenPoolOwner::Creature,
            "zone" => TokenPoolOwner::Zone,
            "battlefield" => TokenPoolOwner::Battlefield,
            value => return Err(format!("line {line}: unknown token pool owner '{value}'")),
        };
        let owner_id = optional(&v, "owner_id");
        if !seen.insert(format!(
            "{game_id}/{owner:?}/{}/{id}",
            owner_id.as_deref().unwrap_or("*"),
            id = pool.id
        )) {
            return Err(format!("line {line}: duplicate token pool assignment"));
        }
        result.push(TokenPoolDefinition {
            pool,
            owner,
            owner_id,
        });
    }
    let ids: HashSet<_> = result.iter().map(|d| d.pool.id.as_str()).collect();
    for d in &result {
        if let Some(parent) = d.pool.parent_id.as_deref() {
            if !ids.contains(parent) {
                return Err(format!(
                    "token pool '{}' has unknown parent '{parent}'",
                    d.pool.id
                ));
            }
        }
    }
    Ok(result)
}

fn pool_from_values(v: &HashMap<String, String>) -> Result<TokenPool, String> {
    let id = optional(v, "id").ok_or("Token pool is missing required id")?;
    let label = optional(v, "label").unwrap_or_else(|| id.clone());
    let token = optional(v, "token")
        .or_else(|| optional(v, "icon"))
        .ok_or_else(|| format!("Token pool '{id}' is missing required icon"))?;
    let starting = number(v, "starting", 0)?;
    let pool = TokenPool {
        id,
        label,
        token_type_id: optional(v, "token_type_id"),
        token,
        icon_color: optional(v, "icon_color"),
        background: optional(v, "background"),
        starting,
        count: starting,
        plus: number(v, "plus", 1)?,
        minus: number(v, "minus", 1)?,
        min: optional_number(v, "min")?,
        max: optional_number(v, "max")?,
        parent_id: optional(v, "parent_id").or_else(|| optional(v, "parent")),
        active: v.get("active").is_some_and(|s| parse_bool(s)),
    };
    pool.validate()?;
    Ok(pool)
}

fn csv_records(input: &str) -> Result<Vec<(usize, HashMap<String, String>)>, String> {
    let mut reader = csv::Reader::from_reader(input.as_bytes());
    let headers: Vec<_> = reader
        .headers()
        .map_err(|e| e.to_string())?
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .collect();
    reader
        .records()
        .enumerate()
        .map(|(i, row)| {
            let row = row.map_err(|e| e.to_string())?;
            Ok((
                i + 2,
                headers
                    .iter()
                    .enumerate()
                    .map(|(j, h)| (h.clone(), row.get(j).unwrap_or("").trim().to_string()))
                    .collect(),
            ))
        })
        .collect()
}
fn required(v: &HashMap<String, String>, key: &str, line: usize) -> Result<String, String> {
    optional(v, key).ok_or_else(|| format!("line {line}: '{key}' is required"))
}
fn optional(v: &HashMap<String, String>, key: &str) -> Option<String> {
    v.get(key)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}
fn number(v: &HashMap<String, String>, key: &str, default: u32) -> Result<u32, String> {
    Ok(optional_number(v, key)?.unwrap_or(default))
}
fn optional_number(v: &HashMap<String, String>, key: &str) -> Result<Option<u32>, String> {
    optional(v, key)
        .map(|s| {
            s.parse()
                .map_err(|_| format!("Invalid token pool {key} '{s}'"))
        })
        .transpose()
}
fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "y"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loads_types_and_csv_pool_assignments() {
        let types = ingest_token_types_csv(
            "game_id,id,name,icon,icon_color\nbyog,red-mana,Red mana,fa-solid fa-mountain,red\n",
        )
        .unwrap();
        let pools = ingest_token_pools_csv("game_id,id,token_type_id,owner,owner_id,starting,plus,minus,active\nbyog,red,red-mana,player,,0,1,1,true\n", &types).unwrap();
        assert_eq!(pools[0].pool.token(), "fa-solid fa-mountain");
        assert_eq!(pools[0].pool.icon_color.as_deref(), Some("red"));
    }
    #[test]
    fn parses_legacy_inline_pool() {
        let pool = TokenPool::parse_list(
            "id=charge|icon=fa-solid fa-bolt|starting=2|min=1|plus=2|minus=1",
        )
        .unwrap()
        .remove(0);
        assert_eq!(pool.starting, 2);
        assert_eq!(pool.plus, 2);
    }
}
