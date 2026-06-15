pub const REPO_SLUG: &str = match option_env!("METEO_REPO_SLUG") {
    Some(slug) => slug,
    None => "ZmoleCristian/numabatevantule",
};

pub const API_BASE: &str = match option_env!("METEO_API_BASE") {
    Some(base) => base,
    None => "https://api.github.com",
};

pub const RAW_BASE: &str = match option_env!("METEO_RAW_BASE") {
    Some(base) => base,
    None => "https://raw.githubusercontent.com",
};
