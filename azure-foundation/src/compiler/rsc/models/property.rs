use crate::compiler::rsc::models::value::Value;

/// Les proprietes CSS "de base" reconnues par rsC : boite (dimensions,
/// marges, padding, bordures), positionnement, arriere-plan/couleur,
/// typographie et flexbox. Ce n'est pas toute la specification CSS - juste
/// le socle annonce, a completer a la demande (grid, transitions,
/// transformations, ...).
///
/// Une declaration dont le nom n'est reconnu par aucune variante ici n'est
/// pas une erreur de parsing (comme en CSS reel, qui ignore les
/// declarations inconnues) : elle est simplement rangee dans
/// `ComputedStyle::custom` plutot que perdue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Property {
    Display,
    Position,
    Top,
    Right,
    Bottom,
    Left,
    Width,
    Height,
    MinWidth,
    MaxWidth,
    MinHeight,
    MaxHeight,
    Margin,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Padding,
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,
    BoxSizing,
    Overflow,
    OverflowY,
    OverflowX,
    ZIndex,
    Visibility,
    Border,
    BorderWidth,
    BorderStyle,
    BorderColor,
    BorderTop,
    BorderRight,
    BorderBottom,
    BorderLeft,
    BorderTopWidth,
    BorderRightWidth,
    BorderBottomWidth,
    BorderLeftWidth,
    BorderRadius,
    Background,
    BackgroundColor,
    BackgroundImage,
    BackgroundSize,
    BackgroundPosition,
    BoxShadow,
    Color,
    FontFamily,
    FontSize,
    FontWeight,
    FontStyle,
    LineHeight,
    TextAlign,
    TextDecoration,
    LetterSpacing,
    WhiteSpace,
    FlexDirection,
    JustifyContent,
    AlignItems,
    AlignContent,
    FlexWrap,
    FlexGrow,
    FlexShrink,
    FlexBasis,
    Gap,
    RowGap,
    ColumnGap,
    AlignSelf,
    /// Liste de pistes (`Value::List` de `Value::Length(_, Unit::Percent | Unit::Fr)`,
    /// voir `layout::models::layout_props::Track`) - pas de support du
    /// raccourci `grid-column: 2 / 4` (le lexer ignore un `/` isole, non
    /// tokenise) : `GridColumn`/`GridRow` ne portent que la piste de depart,
    /// `GridColumnSpan`/`GridRowSpan` portent l'etendue separement.
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridColumnSpan,
    GridRow,
    GridRowSpan,
    Opacity,
    Cursor,
    /// `transition` (raccourci : propriete, duree, courbe, delai).
    Transition,
    /// Couleur des cases cochees, curseurs, barres (champs `Control`).
    AccentColor,
    /// `scrollbar-color: <poignee> [<piste>]` (la piste est ignoree).
    ScrollbarColor,
}

impl Property {
    /// Traduit un nom de propriete CSS en kebab-case (`"font-size"`,
    /// `"z-index"`...) vers sa variante. `None` pour tout ce qui n'est pas
    /// (encore) reconnu.
    pub fn from_name(name: &str) -> Option<Property> {
        use Property::*;
        Some(match name {
            "display" => Display,
            "position" => Position,
            "top" => Top,
            "right" => Right,
            "bottom" => Bottom,
            "left" => Left,
            "width" => Width,
            "height" => Height,
            "min-width" => MinWidth,
            "max-width" => MaxWidth,
            "min-height" => MinHeight,
            "max-height" => MaxHeight,
            "margin" => Margin,
            "margin-top" => MarginTop,
            "margin-right" => MarginRight,
            "margin-bottom" => MarginBottom,
            "margin-left" => MarginLeft,
            "padding" => Padding,
            "padding-top" => PaddingTop,
            "padding-right" => PaddingRight,
            "padding-bottom" => PaddingBottom,
            "padding-left" => PaddingLeft,
            "box-sizing" => BoxSizing,
            "overflow" => Overflow,
            "overflow-y" => OverflowY,
            "overflow-x" => OverflowX,
            "z-index" => ZIndex,
            "visibility" => Visibility,
            "border" => Border,
            "border-width" => BorderWidth,
            "border-style" => BorderStyle,
            "border-color" => BorderColor,
            "border-top" => BorderTop,
            "border-right" => BorderRight,
            "border-bottom" => BorderBottom,
            "border-left" => BorderLeft,
            "border-top-width" => BorderTopWidth,
            "border-right-width" => BorderRightWidth,
            "border-bottom-width" => BorderBottomWidth,
            "border-left-width" => BorderLeftWidth,
            "border-radius" => BorderRadius,
            "background" => Background,
            "background-color" => BackgroundColor,
            "background-image" => BackgroundImage,
            "background-size" => BackgroundSize,
            "background-position" => BackgroundPosition,
            "box-shadow" => BoxShadow,
            "color" => Color,
            "font-family" => FontFamily,
            "font-size" => FontSize,
            "font-weight" => FontWeight,
            "font-style" => FontStyle,
            "line-height" => LineHeight,
            "text-align" => TextAlign,
            "text-decoration" | "text-decoration-line" => TextDecoration,
            "letter-spacing" => LetterSpacing,
            "white-space" => WhiteSpace,
            "flex-direction" => FlexDirection,
            "justify-content" => JustifyContent,
            "align-items" => AlignItems,
            "align-content" => AlignContent,
            "flex-wrap" => FlexWrap,
            "flex-grow" => FlexGrow,
            "flex-shrink" => FlexShrink,
            "flex-basis" => FlexBasis,
            "gap" => Gap,
            "row-gap" => RowGap,
            "column-gap" => ColumnGap,
            "align-self" => AlignSelf,
            "grid-template-columns" => GridTemplateColumns,
            "grid-template-rows" => GridTemplateRows,
            "grid-column" => GridColumn,
            "grid-column-span" => GridColumnSpan,
            "grid-row" => GridRow,
            "grid-row-span" => GridRowSpan,
            "opacity" => Opacity,
            "cursor" => Cursor,
            "transition" => Transition,
            "accent-color" => AccentColor,
            "scrollbar-color" => ScrollbarColor,
            _ => return None,
        })
    }
}

// Un `ComputedStyle` a un champ `Option<Value>` par propriete reconnue.
// Vu le nombre de proprietes, une macro evite de repeter cinq fois (struct,
// new, set, get, merge) la meme liste de noms - et garantit que les cinq
// restent synchronises.
macro_rules! computed_style {
    ($( $variant:ident => $field:ident ),+ $(,)?) => {
        /// L'ensemble des valeurs de proprietes resolues pour un element -
        /// l'equivalent d'un bloc de regles CSS "aplati" par la cascade.
        /// Chaque champ est optionnel pour permettre la fusion (`merge`) :
        /// un style partiel peut etre applique par-dessus un autre sans
        /// ecraser le reste, exactement comme `style::models::style::Style`
        /// mais avec la totalite des proprietes de base plutot qu'un
        /// sous-ensemble fixe.
        #[derive(Debug, Clone, Default, PartialEq)]
        pub struct ComputedStyle {
            $( pub $field: Option<Value>, )+
            /// Declarations dont la propriete n'est pas reconnue : gardees
            /// brutes (nom, valeur affichee) plutot que perdues.
            pub custom: Vec<(String, String)>,
        }

        impl ComputedStyle {
            pub fn new() -> Self {
                Self::default()
            }

            pub fn set(&mut self, property: Property, value: Value) {
                match property {
                    $( Property::$variant => self.$field = Some(value), )+
                }
            }

            pub fn get(&self, property: Property) -> Option<&Value> {
                match property {
                    $( Property::$variant => self.$field.as_ref(), )+
                }
            }

            /// Fusionne `other` par-dessus `self`, propriete par propriete :
            /// meme semantique de cascade que
            /// `style::models::style::Style::merge`.
            pub fn merge(&self, other: &ComputedStyle) -> ComputedStyle {
                ComputedStyle {
                    $( $field: other.$field.clone().or_else(|| self.$field.clone()), )+
                    custom: self.custom.iter().cloned().chain(other.custom.iter().cloned()).collect(),
                }
            }
        }
    };
}

computed_style! {
    Display => display,
    Position => position,
    Top => top,
    Right => right,
    Bottom => bottom,
    Left => left,
    Width => width,
    Height => height,
    MinWidth => min_width,
    MaxWidth => max_width,
    MinHeight => min_height,
    MaxHeight => max_height,
    Margin => margin,
    MarginTop => margin_top,
    MarginRight => margin_right,
    MarginBottom => margin_bottom,
    MarginLeft => margin_left,
    Padding => padding,
    PaddingTop => padding_top,
    PaddingRight => padding_right,
    PaddingBottom => padding_bottom,
    PaddingLeft => padding_left,
    BoxSizing => box_sizing,
    Overflow => overflow,
    OverflowY => overflow_y,
    OverflowX => overflow_x,
    ZIndex => z_index,
    Visibility => visibility,
    Border => border,
    BorderWidth => border_width,
    BorderStyle => border_style,
    BorderColor => border_color,
    BorderTop => border_top,
    BorderRight => border_right,
    BorderBottom => border_bottom,
    BorderLeft => border_left,
    BorderTopWidth => border_top_width,
    BorderRightWidth => border_right_width,
    BorderBottomWidth => border_bottom_width,
    BorderLeftWidth => border_left_width,
    BorderRadius => border_radius,
    Background => background,
    BackgroundColor => background_color,
    BackgroundImage => background_image,
    BackgroundSize => background_size,
    BackgroundPosition => background_position,
    BoxShadow => box_shadow,
    Color => color,
    FontFamily => font_family,
    FontSize => font_size,
    FontWeight => font_weight,
    FontStyle => font_style,
    LineHeight => line_height,
    TextAlign => text_align,
    TextDecoration => text_decoration,
    LetterSpacing => letter_spacing,
    WhiteSpace => white_space,
    FlexDirection => flex_direction,
    JustifyContent => justify_content,
    AlignItems => align_items,
    AlignContent => align_content,
    FlexWrap => flex_wrap,
    FlexGrow => flex_grow,
    FlexShrink => flex_shrink,
    FlexBasis => flex_basis,
    Gap => gap,
    RowGap => row_gap,
    ColumnGap => column_gap,
    AlignSelf => align_self,
    GridTemplateColumns => grid_template_columns,
    GridTemplateRows => grid_template_rows,
    GridColumn => grid_column,
    GridColumnSpan => grid_column_span,
    GridRow => grid_row,
    GridRowSpan => grid_row_span,
    Opacity => opacity,
    Cursor => cursor,
    Transition => transition,
    AccentColor => accent_color,
    ScrollbarColor => scrollbar_color,
}
