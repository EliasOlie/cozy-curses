//! pixelwind — "Tailwind para pixel art".
//!
//! Um tema (`theme.toml`) define paletas estritas com chaves semânticas.
//! Um asset (`*.json`) descreve o sprite só com essas chaves, em camadas.
//! O renderer traduz chaves -> cores, aplica o contorno e gera PNGs com alpha.

use std::collections::BTreeMap;
use std::fmt;

use image::{Rgba, RgbaImage};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Tema
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct Theme {
    pub grid: Grid,
    pub border: Border,
    pub day_palette: BTreeMap<String, String>,
    pub night_palette: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Grid {
    pub tile_width: u32,
    pub tile_height: u32,
    pub sprite_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct Border {
    pub enabled: bool,
    /// Chave da paleta usada no contorno.
    pub color: String,
    pub thickness: u32,
}

impl Theme {
    pub fn parse(src: &str) -> Result<Self, Error> {
        let theme: Theme = toml::from_str(src).map_err(|e| Error(format!("tema inválido: {e}")))?;
        theme.validate()?;
        Ok(theme)
    }

    fn validate(&self) -> Result<(), Error> {
        let day: Vec<_> = self.day_palette.keys().collect();
        let night: Vec<_> = self.night_palette.keys().collect();
        if day != night {
            let only_day: Vec<_> = day.iter().filter(|k| !night.contains(k)).collect();
            let only_night: Vec<_> = night.iter().filter(|k| !day.contains(k)).collect();
            return Err(Error(format!(
                "as paletas precisam ter as mesmas chaves (dualidade dia/noite). \
                 Só no dia: {only_day:?}. Só na noite: {only_night:?}"
            )));
        }
        for phase in [Phase::Day, Phase::Night] {
            for (key, hex) in self.palette(phase) {
                parse_hex(hex).map_err(|e| Error(format!("{phase}.{key}: {e}")))?;
            }
        }
        if !self.day_palette.contains_key(&self.border.color) {
            return Err(Error(format!(
                "border.color = {:?} não existe nas paletas",
                self.border.color
            )));
        }
        Ok(())
    }

    pub fn palette(&self, phase: Phase) -> &BTreeMap<String, String> {
        match phase {
            Phase::Day => &self.day_palette,
            Phase::Night => &self.night_palette,
        }
    }

    pub fn color(&self, phase: Phase, key: &str) -> Result<Rgba<u8>, Error> {
        let hex = self.palette(phase).get(key).ok_or_else(|| {
            let keys: Vec<_> = self.day_palette.keys().map(String::as_str).collect();
            Error(format!("cor {key:?} não existe no tema. Chaves válidas: {}", keys.join(", ")))
        })?;
        parse_hex(hex)
    }
}

pub fn parse_hex(hex: &str) -> Result<Rgba<u8>, Error> {
    let h = hex.strip_prefix('#').ok_or_else(|| Error(format!("{hex:?} deve começar com #")))?;
    let byte = |i: usize| {
        u8::from_str_radix(&h[i..i + 2], 16).map_err(|_| Error(format!("hex inválido: {hex:?}")))
    };
    match h.len() {
        6 => Ok(Rgba([byte(0)?, byte(2)?, byte(4)?, 255])),
        8 => Ok(Rgba([byte(0)?, byte(2)?, byte(4)?, byte(6)?])),
        _ => Err(Error(format!("hex inválido: {hex:?} (use #rrggbb ou #rrggbbaa)"))),
    }
}

// ---------------------------------------------------------------------------
// Asset declarativo
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Day,
    Night,
}

impl fmt::Display for Phase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Phase::Day => "day",
            Phase::Night => "night",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaletteMode {
    Day,
    Night,
    /// Gera `<nome>_day.png` e `<nome>_night.png`.
    Both,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    pub name: String,
    /// [largura, altura]
    pub size: [u32; 2],
    pub palette: PaletteMode,
    /// Sobrescreve `border.enabled` do tema (tiles de chão costumam usar false).
    #[serde(default)]
    pub outline: Option<bool>,
    /// Caractere -> chave da paleta. `null` = transparente.
    /// `.` e espaço são sempre transparentes.
    #[serde(default)]
    pub legend: BTreeMap<String, Option<String>>,
    /// Camadas desenhadas em todo quadro (ou o desenho inteiro, sem `frames`).
    #[serde(default)]
    pub layers: Vec<Layer>,
    /// Spritesheet: cada quadro tem o tamanho `size` e eles são dispostos em grade,
    /// da esquerda para a direita, `columns` por linha.
    #[serde(default)]
    pub frames: Vec<Frame>,
    /// Quadros por linha da spritesheet. Padrão: todos numa linha só.
    #[serde(default)]
    pub columns: Option<u32>,
    /// Texto livre (o que é, de onde veio a referência). Ignorado pelo renderer.
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub name: String,
    /// Herda tudo o que o quadro citado desenha (útil para passos de caminhada).
    #[serde(default)]
    pub from: Option<String>,
    /// Desloca o que é herdado (camadas base + `from`), não as camadas do próprio quadro.
    #[serde(default)]
    pub offset: [i32; 2],
    #[serde(default)]
    pub layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    #[serde(default)]
    pub name: Option<String>,
    /// Camada só existe nesta fase (ex.: olhos que aparecem só à noite).
    #[serde(default)]
    pub only: Option<Phase>,
    /// Deslocamento [x, y] das linhas da camada.
    #[serde(default)]
    pub offset: [i32; 2],
    /// Matriz de pixels: cada string é uma linha, cada caractere uma chave da legenda.
    #[serde(default)]
    pub rows: Option<Vec<String>>,
    /// Simetria: cada linha é a metade esquerda e é espelhada para a direita.
    #[serde(default)]
    pub mirror: bool,
    /// Forma geométrica: "rect", "ellipse" ou "iso_diamond".
    #[serde(default)]
    pub shape: Option<ShapeKind>,
    /// [x, y, largura, altura] da forma. Padrão: o asset inteiro.
    #[serde(default)]
    pub rect: Option<[i32; 4]>,
    /// Cor de preenchimento da forma (sem cor = só o ruído é pintado).
    #[serde(default)]
    pub color: Option<String>,
    /// Pontilhado determinístico aplicado dentro da forma.
    #[serde(default)]
    pub noise: Vec<Noise>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    Rect,
    Ellipse,
    IsoDiamond,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Noise {
    pub color: String,
    /// 0.0..=1.0 — fração dos pixels da forma que recebem a cor.
    pub density: f32,
    #[serde(default)]
    pub seed: u32,
}

impl Asset {
    pub fn parse(src: &str) -> Result<Self, Error> {
        serde_json::from_str(src).map_err(|e| Error(format!("asset inválido: {e}")))
    }

    pub fn phases(&self) -> Vec<Phase> {
        match self.palette {
            PaletteMode::Day => vec![Phase::Day],
            PaletteMode::Night => vec![Phase::Night],
            PaletteMode::Both => vec![Phase::Day, Phase::Night],
        }
    }

    /// Nome do arquivo de saída (sem diretório) para a fase.
    pub fn file_name(&self, phase: Phase) -> String {
        match self.palette {
            PaletteMode::Both => format!("{}_{phase}.png", self.name),
            _ => format!("{}.png", self.name),
        }
    }
}

// ---------------------------------------------------------------------------
// Renderização
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// Canvas de chaves de paleta: o contorno é calculado antes de virar cor,
/// então ele respeita a paleta da fase.
struct Canvas {
    w: u32,
    h: u32,
    px: Vec<Option<String>>,
}

impl Canvas {
    fn new(w: u32, h: u32) -> Self {
        Self { w, h, px: vec![None; (w * h) as usize] }
    }

    fn set(&mut self, x: i32, y: i32, key: Option<&str>) {
        if x >= 0 && y >= 0 && (x as u32) < self.w && (y as u32) < self.h {
            if let Some(key) = key {
                self.px[(y as u32 * self.w + x as u32) as usize] = Some(key.to_owned());
            }
        }
    }

    fn get(&self, x: i32, y: i32) -> Option<&str> {
        if x < 0 || y < 0 || x as u32 >= self.w || y as u32 >= self.h {
            return None;
        }
        self.px[(y as u32 * self.w + x as u32) as usize].as_deref()
    }

    /// Pinta `key` em todo pixel vazio vizinho (4-direções) de um pixel cheio.
    fn outline(&mut self, key: &str, thickness: u32) {
        for _ in 0..thickness {
            let mut ring = Vec::new();
            for y in 0..self.h as i32 {
                for x in 0..self.w as i32 {
                    if self.get(x, y).is_none()
                        && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                            .iter()
                            .any(|(dx, dy)| self.get(x + dx, y + dy).is_some())
                    {
                        ring.push((x, y));
                    }
                }
            }
            for (x, y) in ring {
                self.set(x, y, Some(key));
            }
        }
    }
}

/// Hash determinístico (mesmo asset => mesmo pontilhado, sempre).
fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ seed.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    (h & 0xFFFF) as f32 / 65536.0
}

fn inside(shape: ShapeKind, px: f32, py: f32, [x, y, w, h]: [i32; 4]) -> bool {
    let (x, y, w, h) = (x as f32, y as f32, w as f32, h as f32);
    match shape {
        ShapeKind::Rect => px >= x && px < x + w && py >= y && py < y + h,
        ShapeKind::Ellipse | ShapeKind::IsoDiamond => {
            let (cx, cy, rx, ry) = (x + w / 2.0, y + h / 2.0, w / 2.0, h / 2.0);
            let (dx, dy) = ((px - cx) / rx, (py - cy) / ry);
            if shape == ShapeKind::Ellipse {
                dx * dx + dy * dy <= 1.0
            } else {
                dx.abs() + dy.abs() <= 1.0
            }
        }
    }
}

/// Camadas que um quadro desenha, cada uma com o deslocamento acumulado.
fn frame_layers<'a>(
    asset: &'a Asset,
    frame: &'a Frame,
    depth: usize,
) -> Result<Vec<(&'a Layer, [i32; 2])>, Error> {
    if depth > asset.frames.len() {
        return Err(Error(format!("quadro {}: `from` em ciclo", frame.name)));
    }
    let mut inherited: Vec<(&Layer, [i32; 2])> = match &frame.from {
        Some(from) => {
            let parent = asset.frames.iter().find(|f| &f.name == from).ok_or_else(|| {
                Error(format!("quadro {}: `from` aponta para {from:?}, que não existe", frame.name))
            })?;
            frame_layers(asset, parent, depth + 1)?
        }
        None => asset.layers.iter().map(|l| (l, [0, 0])).collect(),
    };
    for (_, off) in &mut inherited {
        off[0] += frame.offset[0];
        off[1] += frame.offset[1];
    }
    inherited.extend(frame.layers.iter().map(|l| (l, [0, 0])));
    Ok(inherited)
}

pub fn render(theme: &Theme, asset: &Asset, phase: Phase) -> Result<RgbaImage, Error> {
    let [w, h] = asset.size;
    if w == 0 || h == 0 {
        return Err(Error("size precisa ser maior que zero".into()));
    }
    if asset.frames.is_empty() {
        let layers: Vec<_> = asset.layers.iter().map(|l| (l, [0, 0])).collect();
        return render_frame(theme, asset, &layers, phase);
    }

    let columns = asset.columns.unwrap_or(asset.frames.len() as u32).max(1);
    let rows = (asset.frames.len() as u32).div_ceil(columns);
    let mut sheet = RgbaImage::new(w * columns, h * rows);
    for (i, frame) in asset.frames.iter().enumerate() {
        let layers = frame_layers(asset, frame, 0)?;
        let img = render_frame(theme, asset, &layers, phase)
            .map_err(|e| Error(format!("quadro {}: {e}", frame.name)))?;
        let (col, row) = (i as u32 % columns, i as u32 / columns);
        image::imageops::overlay(&mut sheet, &img, (col * w) as i64, (row * h) as i64);
    }
    Ok(sheet)
}

fn render_frame(
    theme: &Theme,
    asset: &Asset,
    layers: &[(&Layer, [i32; 2])],
    phase: Phase,
) -> Result<RgbaImage, Error> {
    let [w, h] = asset.size;
    let mut canvas = Canvas::new(w, h);

    for (i, (layer, shift)) in layers.iter().enumerate() {
        if layer.only.is_some_and(|only| only != phase) {
            continue;
        }
        let label = layer.name.clone().unwrap_or_else(|| format!("#{i}"));
        let ctx = |msg: String| Error(format!("camada {label}: {msg}"));
        let (dx, dy) = (layer.offset[0] + shift[0], layer.offset[1] + shift[1]);

        match (&layer.rows, layer.shape) {
            (Some(rows), None) => {
                let mirrored: Vec<String>;
                let rows = if layer.mirror {
                    mirrored = rows
                        .iter()
                        .map(|r| r.chars().chain(r.chars().rev()).collect())
                        .collect();
                    &mirrored
                } else {
                    rows
                };
                let width = rows.first().map_or(0, |r| r.chars().count());
                for (y, row) in rows.iter().enumerate() {
                    let n = row.chars().count();
                    if n != width {
                        return Err(ctx(format!(
                            "linha {y} tem {n} colunas, a primeira tem {width}"
                        )));
                    }
                    for (x, ch) in row.chars().enumerate() {
                        let key = if ch == '.' || ch == ' ' {
                            None
                        } else {
                            match asset.legend.get(&ch.to_string()) {
                                Some(key) => key.as_deref(),
                                None => {
                                    return Err(ctx(format!(
                                        "caractere {ch:?} (linha {y}, coluna {x}) não está na legenda"
                                    )));
                                }
                            }
                        };
                        canvas.set(x as i32 + dx, y as i32 + dy, key);
                    }
                }
            }
            (None, Some(shape)) => {
                let [rx, ry, rw, rh] = layer.rect.unwrap_or([0, 0, w as i32, h as i32]);
                let rect = [rx + dx, ry + dy, rw, rh];
                for y in 0..h as i32 {
                    for x in 0..w as i32 {
                        if !inside(shape, x as f32 + 0.5, y as f32 + 0.5, rect) {
                            continue;
                        }
                        let mut key = layer.color.as_deref();
                        for noise in &layer.noise {
                            if hash01(x, y, noise.seed) < noise.density {
                                key = Some(&noise.color);
                            }
                        }
                        canvas.set(x, y, key);
                    }
                }
            }
            _ => return Err(ctx("use exatamente um de `rows` ou `shape`".into())),
        }
    }

    if asset.outline.unwrap_or(theme.border.enabled) {
        canvas.outline(&theme.border.color, theme.border.thickness);
    }

    let mut img = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            if let Some(key) = canvas.get(x as i32, y as i32) {
                img.put_pixel(x, y, theme.color(phase, key)?);
            }
        }
    }
    Ok(img)
}

/// Amplia sem interpolação (para pré-visualização).
pub fn upscale(img: &RgbaImage, factor: u32) -> RgbaImage {
    if factor <= 1 {
        return img.clone();
    }
    RgbaImage::from_fn(img.width() * factor, img.height() * factor, |x, y| {
        *img.get_pixel(x / factor, y / factor)
    })
}

/// Cartela com as duas paletas (linha de cima: dia, de baixo: noite).
pub fn swatch(theme: &Theme) -> Result<RgbaImage, Error> {
    let keys: Vec<_> = theme.day_palette.keys().collect();
    let cell = 16;
    let mut img = RgbaImage::new(keys.len() as u32 * cell, cell * 2);
    for (i, key) in keys.iter().enumerate() {
        for (row, phase) in [Phase::Day, Phase::Night].into_iter().enumerate() {
            let c = theme.color(phase, key)?;
            for y in 0..cell {
                for x in 0..cell {
                    img.put_pixel(i as u32 * cell + x, row as u32 * cell + y, c);
                }
            }
        }
    }
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEME: &str = r##"
        [grid]
        tile_width = 32
        tile_height = 16
        sprite_size = 32
        [border]
        enabled = true
        color = "outline"
        thickness = 1
        [day_palette]
        outline = "#000000"
        stone = "#808080"
        eye = "#ff0000"
        [night_palette]
        outline = "#100010"
        stone = "#404060"
        eye = "#00ff00"
    "##;

    fn theme() -> Theme {
        Theme::parse(THEME).unwrap()
    }

    #[test]
    fn traduz_chaves_e_aplica_contorno() {
        let asset = Asset::parse(
            r#"{"name":"t","size":[3,3],"palette":"night",
                "legend":{"s":"stone"},"layers":[{"rows":["...",".s.","..."]}]}"#,
        )
        .unwrap();
        let img = render(&theme(), &asset, Phase::Night).unwrap();
        assert_eq!(img.get_pixel(1, 1), &Rgba([0x40, 0x40, 0x60, 255]));
        assert_eq!(img.get_pixel(1, 0), &Rgba([0x10, 0x00, 0x10, 255]));
        // diagonal não recebe contorno (4-vizinhança)
        assert_eq!(img.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn camada_only_respeita_a_fase() {
        let asset = Asset::parse(
            r#"{"name":"t","size":[1,1],"palette":"both","outline":false,
                "legend":{"s":"stone","e":"eye"},
                "layers":[{"rows":["s"]},{"only":"night","rows":["e"]}]}"#,
        )
        .unwrap();
        assert_eq!(render(&theme(), &asset, Phase::Day).unwrap().get_pixel(0, 0)[0], 0x80);
        assert_eq!(render(&theme(), &asset, Phase::Night).unwrap().get_pixel(0, 0)[1], 0xff);
    }

    #[test]
    fn mirror_espelha_a_metade_esquerda() {
        let asset = Asset::parse(
            r#"{"name":"t","size":[4,1],"palette":"day","outline":false,
                "legend":{"s":"stone","e":"eye"},"layers":[{"mirror":true,"rows":["se"]}]}"#,
        )
        .unwrap();
        let img = render(&theme(), &asset, Phase::Day).unwrap();
        assert_eq!(img.get_pixel(0, 0), img.get_pixel(3, 0));
        assert_eq!(img.get_pixel(1, 0), img.get_pixel(2, 0));
        assert_ne!(img.get_pixel(0, 0), img.get_pixel(1, 0));
    }

    #[test]
    fn frames_montam_spritesheet_com_heranca() {
        let asset = Asset::parse(
            r#"{"name":"t","size":[2,2],"palette":"day","outline":false,"columns":2,
                "legend":{"s":"stone","e":"eye"},
                "layers":[{"rows":["s"]}],
                "frames":[
                    {"name":"a"},
                    {"name":"b","from":"a","offset":[1,1]},
                    {"name":"c","from":"b","layers":[{"rows":["e"]}]}
                ]}"#,
        )
        .unwrap();
        let img = render(&theme(), &asset, Phase::Day).unwrap();
        assert_eq!((img.width(), img.height()), (4, 4));
        let stone = Rgba([0x80, 0x80, 0x80, 255]);
        assert_eq!(img.get_pixel(0, 0), &stone); // a: base em (0,0)
        assert_eq!(img.get_pixel(3, 1), &stone); // b: base deslocada para (1,1)
        assert_eq!(img.get_pixel(1, 3), &stone); // c: herda b...
        assert_eq!(img.get_pixel(0, 2)[0], 0xff); // ...e soma a própria camada, sem deslocar
    }

    #[test]
    fn erros_sao_legiveis() {
        let asset = Asset::parse(
            r#"{"name":"t","size":[2,1],"palette":"day","legend":{},"layers":[{"rows":["x."]}]}"#,
        )
        .unwrap();
        let err = render(&theme(), &asset, Phase::Day).unwrap_err();
        assert!(err.0.contains("não está na legenda"), "{err}");
    }

    #[test]
    fn paletas_precisam_ter_as_mesmas_chaves() {
        let broken = THEME.replace("eye = \"#00ff00\"", "");
        assert!(Theme::parse(&broken).unwrap_err().0.contains("mesmas chaves"));
    }
}
