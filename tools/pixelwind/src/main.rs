use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pixelwind::{Asset, Theme, render, swatch, upscale};

const USAGE: &str = "\
pixelwind — Tailwind para pixel art

USO:
    pixelwind [opções] <asset.json | pasta>...

OPÇÕES:
    --theme <arquivo>   tema TOML (padrão: art/theme.toml)
    --out <pasta>       onde salvar os PNGs (padrão: assets/sprites)
    --scale <n>         amplia sem interpolação, para pré-visualizar (padrão: 1)
    --check             só valida, não grava nada
    --swatch            grava <out>/palette.png com as duas paletas
    -h, --help          mostra esta ajuda

Assets com \"palette\": \"both\" geram <nome>_day.png e <nome>_night.png.";

struct Args {
    theme: PathBuf,
    out: PathBuf,
    scale: u32,
    check: bool,
    swatch: bool,
    inputs: Vec<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        theme: "art/theme.toml".into(),
        out: "assets/sprites".into(),
        scale: 1,
        check: false,
        swatch: false,
        inputs: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} precisa de um valor"));
        match arg.as_str() {
            "--theme" => args.theme = value("--theme")?.into(),
            "--out" => args.out = value("--out")?.into(),
            "--scale" => {
                args.scale = value("--scale")?
                    .parse()
                    .map_err(|_| "--scale precisa ser um inteiro".to_string())?
            }
            "--check" => args.check = true,
            "--swatch" => args.swatch = true,
            "-h" | "--help" => return Err(USAGE.into()),
            s if s.starts_with('-') => return Err(format!("opção desconhecida: {s}\n\n{USAGE}")),
            _ => args.inputs.push(arg.into()),
        }
    }
    if args.inputs.is_empty() && !args.swatch {
        return Err(USAGE.into());
    }
    Ok(args)
}

fn collect(inputs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for input in inputs {
        if input.is_dir() {
            let entries = std::fs::read_dir(input).map_err(|e| format!("{}: {e}", input.display()))?;
            let mut found: Vec<_> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect();
            found.sort();
            files.extend(found);
        } else {
            files.push(input.clone());
        }
    }
    Ok(files)
}

fn build(theme: &Theme, file: &Path, args: &Args) -> Result<Vec<PathBuf>, String> {
    let src = std::fs::read_to_string(file).map_err(|e| e.to_string())?;
    let asset = Asset::parse(&src).map_err(|e| e.to_string())?;
    let mut written = Vec::new();
    for phase in asset.phases() {
        let img = render(theme, &asset, phase).map_err(|e| format!("[{phase}] {e}"))?;
        if args.check {
            continue;
        }
        let path = args.out.join(asset.file_name(phase));
        upscale(&img, args.scale).save(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let theme_src = std::fs::read_to_string(&args.theme)
        .map_err(|e| format!("{}: {e}", args.theme.display()))?;
    let theme = Theme::parse(&theme_src).map_err(|e| e.to_string())?;
    if !args.check {
        std::fs::create_dir_all(&args.out).map_err(|e| format!("{}: {e}", args.out.display()))?;
    }

    if args.swatch {
        let path = args.out.join("palette.png");
        let img = upscale(&swatch(&theme).map_err(|e| e.to_string())?, args.scale);
        img.save(&path).map_err(|e| e.to_string())?;
        println!("✔ {}", path.display());
    }

    let mut failed = 0;
    for file in collect(&args.inputs)? {
        match build(&theme, &file, &args) {
            Ok(_) if args.check => println!("✔ {} ok", file.display()),
            Ok(paths) => {
                for p in paths {
                    println!("✔ {} -> {}", file.display(), p.display());
                }
            }
            Err(e) => {
                eprintln!("✘ {}: {e}", file.display());
                failed += 1;
            }
        }
    }
    if failed > 0 {
        return Err(format!("{failed} asset(s) com erro"));
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
