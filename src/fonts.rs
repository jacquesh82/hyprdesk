//! Police livrée avec hyprdesk, disponible sans dépendre du cache Fontconfig.
use std::borrow::Cow;

pub const FAMILY: &str = "Mindlog";
pub const DEFAULT: iced::Font = iced::Font::with_name(FAMILY);

pub const FACES: [&[u8]; 5] = [
    include_bytes!("../assets/fonts/mindlog/Mindlog-Light.ttf"),
    include_bytes!("../assets/fonts/mindlog/Mindlog-Regular.ttf"),
    include_bytes!("../assets/fonts/mindlog/Mindlog-Medium.ttf"),
    include_bytes!("../assets/fonts/mindlog/Mindlog-SemiBold.ttf"),
    include_bytes!("../assets/fonts/mindlog/Mindlog-Bold.ttf"),
];

pub fn bundled() -> Vec<Cow<'static, [u8]>> {
    FACES.into_iter().map(Cow::Borrowed).collect()
}
