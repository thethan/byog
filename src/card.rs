#[derive(Debug, Clone)]
pub struct Card {
    pub game_id: String,
    pub id: String,
    pub name: String,
    pub card_type_id: String,
    pub description: Option<String>,
    pub cost: Option<CardCost>,
    pub visual: CardVisual,
    pub back_logo: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CardVisual {
    /// Application-generated card layout.
    Generated {
        /// Main card artwork.
        image: Option<String>,

        /// Card background, potentially inherited from CardType.
        background_image: Option<String>,

        background_color: Option<String>,
        icon: Option<String>,
    },

    /// Complete card image with no generated content over it.
    FullImage {
        image: String,
    },
}