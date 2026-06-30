mod pix;

use image::{GrayImage, Luma};
use qrcode::render::unicode;
use qrcode::{EcLevel, QrCode, Version};
use std::env;
use std::fs::File;
use std::io::{self, Write};

/// Configuração do gerador de QR Code
struct Configuracao {
    /// Dados a serem codificados (texto ou URL)
    dados: String,
    /// Formato de saída desejado
    formato: FormatoSaida,
    /// Nível de correção de erros
    nivel_ec: EcLevel,
    /// Tamanho de cada módulo em pixels (para PNG)
    tamanho_modulo: u32,
    /// Largura da margem em módulos (quiet zone)
    margem: u32,
    /// Caminho do arquivo de saída (quando aplicável)
    caminho_saida: Option<String>,
    /// Caminho para o arquivo de configuração PIX (quando modo PIX ativo)
    pix_config: Option<String>,
}

/// Formatos de saída suportados
#[derive(Debug, Clone, PartialEq)]
enum FormatoSaida {
    Terminal,
    Svg,
    Png,
}

/// Renderiza o QR Code no terminal usando caracteres Unicode
fn renderizar_terminal(codigo: &QrCode) {
    // A crate qrcode tem suporte nativo para renderização Unicode
    let texto = codigo
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .quiet_zone(true)
        .build();

    println!("{}", texto);
}

/// Gera um arquivo SVG a partir do QR Code
fn gerar_svg(codigo: &QrCode, caminho: &str, tamanho_modulo: u32, margem: u32) -> io::Result<()> {
    let matriz = codigo.to_colors();
    let largura_qr = codigo.width() as u32;

    // Dimensões totais do SVG incluindo margens
    let tamanho_total = (largura_qr + margem * 2) * tamanho_modulo;

    let mut svg = String::new();

    // Cabeçalho SVG com namespace
    svg.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" version="1.1" width="{t}" height="{t}" viewBox="0 0 {t} {t}">
"#,
        t = tamanho_total
    ));

    // Fundo branco
    svg.push_str(&format!(
        r#"  <rect width="{}" height="{}" fill="white"/>
"#,
        tamanho_total, tamanho_total
    ));

    // Desenha cada módulo escuro como um retângulo preto
    for y in 0..largura_qr {
        for x in 0..largura_qr {
            let indice = (y * largura_qr + x) as usize;
            if matriz[indice] == qrcode::Color::Dark {
                let px = (x + margem) * tamanho_modulo;
                let py = (y + margem) * tamanho_modulo;
                svg.push_str(&format!(
                    r#"  <rect x="{}" y="{}" width="{}" height="{}" fill="black"/>
"#,
                    px, py, tamanho_modulo, tamanho_modulo
                ));
            }
        }
    }

    svg.push_str("</svg>\n");

    // Escreve o arquivo SVG
    let mut arquivo = File::create(caminho)?;
    arquivo.write_all(svg.as_bytes())?;

    println!(
        "SVG salvo em '{}' ({}x{} pixels)",
        caminho, tamanho_total, tamanho_total
    );

    Ok(())
}

/// Gera um arquivo PNG a partir do QR Code
fn gerar_png(codigo: &QrCode, caminho: &str, tamanho_modulo: u32, margem: u32) -> io::Result<()> {
    let matriz = codigo.to_colors();
    let largura_qr = codigo.width() as u32;

    // Dimensões totais da imagem incluindo margens
    let tamanho_total = (largura_qr + margem * 2) * tamanho_modulo;

    // Cria a imagem em escala de cinza, toda branca
    let mut imagem = GrayImage::from_pixel(tamanho_total, tamanho_total, Luma([255u8]));

    // Preenche os módulos escuros com preto
    for y in 0..largura_qr {
        for x in 0..largura_qr {
            let indice = (y * largura_qr + x) as usize;
            if matriz[indice] == qrcode::Color::Dark {
                // Preenche o bloco de pixels correspondente ao módulo
                let px_inicio = (x + margem) * tamanho_modulo;
                let py_inicio = (y + margem) * tamanho_modulo;

                for dy in 0..tamanho_modulo {
                    for dx in 0..tamanho_modulo {
                        imagem.put_pixel(
                            px_inicio + dx,
                            py_inicio + dy,
                            Luma([0u8]), // Preto
                        );
                    }
                }
            }
        }
    }

    // Salva a imagem como PNG
    imagem
        .save(caminho)
        .map_err(|e| io::Error::other(format!("Erro ao salvar PNG: {}", e)))?;

    println!(
        "PNG salvo em '{}' ({}x{} pixels)",
        caminho, tamanho_total, tamanho_total
    );

    Ok(())
}

/// Exibe informações sobre o QR Code gerado
fn exibir_info(codigo: &QrCode, dados: &str, nivel_ec: EcLevel) {
    let versao = match codigo.version() {
        Version::Normal(v) => format!("{}", v),
        Version::Micro(v) => format!("M{}", v),
    };
    let nivel_str = match nivel_ec {
        EcLevel::L => "Low (7%)",
        EcLevel::M => "Medium (15%)",
        EcLevel::Q => "Quartile (25%)",
        EcLevel::H => "High (30%)",
    };

    println!("--- Informações do QR Code ---");
    println!(
        "  Dados:              {} ({} caracteres)",
        truncar(dados, 50),
        dados.len()
    );
    println!("  Versão:             {}", versao);
    println!(
        "  Módulos:            {}x{}",
        codigo.width(),
        codigo.width()
    );
    println!("  Correção de erros:  {}", nivel_str);
    println!();
}

/// Trunca uma string adicionando reticências se exceder o limite
fn truncar(texto: &str, limite: usize) -> String {
    if texto.len() <= limite {
        texto.to_string()
    } else {
        format!("{}...", &texto[..limite])
    }
}

/// Analisa os argumentos da linha de comando
fn analisar_argumentos() -> Result<Configuracao, String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        return Err(exibir_uso());
    }

    let mut dados = String::new();
    let mut formato = FormatoSaida::Terminal;
    let mut nivel_ec = EcLevel::M;
    let mut tamanho_modulo = 10;
    let mut margem = 4;
    let mut caminho_saida = None;
    let mut pix_config: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--formato" | "-f" => {
                i += 1;
                if i >= args.len() {
                    return Err("Falta o valor para --formato".to_string());
                }
                formato = match args[i].as_str() {
                    "terminal" | "t" => FormatoSaida::Terminal,
                    "svg" | "s" => FormatoSaida::Svg,
                    "png" | "p" => FormatoSaida::Png,
                    outro => return Err(format!("Formato desconhecido: '{}'", outro)),
                };
            }
            "--ec" | "-e" => {
                i += 1;
                if i >= args.len() {
                    return Err("Falta o valor para --ec".to_string());
                }
                nivel_ec = match args[i].as_str() {
                    "L" | "low" => EcLevel::L,
                    "M" | "medium" => EcLevel::M,
                    "Q" | "quartile" => EcLevel::Q,
                    "H" | "high" => EcLevel::H,
                    outro => return Err(format!("Nível EC desconhecido: '{}'", outro)),
                };
            }
            "--tamanho" | "-t" => {
                i += 1;
                if i >= args.len() {
                    return Err("Falta o valor para --tamanho".to_string());
                }
                tamanho_modulo = args[i].parse::<u32>().map_err(|_| {
                    "O valor de --tamanho deve ser um número inteiro positivo".to_string()
                })?;
            }
            "--margem" | "-m" => {
                i += 1;
                if i >= args.len() {
                    return Err("Falta o valor para --margem".to_string());
                }
                margem = args[i].parse::<u32>().map_err(|_| {
                    "O valor de --margem deve ser um número inteiro positivo".to_string()
                })?;
            }
            "--saida" | "-o" => {
                i += 1;
                if i >= args.len() {
                    return Err("Falta o valor para --saida".to_string());
                }
                caminho_saida = Some(args[i].clone());
            }
            "--pix" | "-p" => {
                // Optional path to PIX config file (default: config_pix.ron)
                if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                    i += 1;
                    pix_config = Some(args[i].clone());
                } else {
                    pix_config = Some("config_pix.ron".to_string());
                }
            }
            outro => {
                if outro.starts_with('-') {
                    return Err(format!("Opção desconhecida: '{}'", outro));
                }
                dados = outro.to_string();
            }
        }
        i += 1;
    }

    if dados.is_empty() && pix_config.is_none() {
        return Err(
            "Nenhum dado fornecido para codificar. Use --pix ou forneça um texto.".to_string(),
        );
    }

    // Define caminho padrão de saída se necessário
    if caminho_saida.is_none() && formato != FormatoSaida::Terminal {
        caminho_saida = Some(match formato {
            FormatoSaida::Svg => "qrcode.svg".to_string(),
            FormatoSaida::Png => "qrcode.png".to_string(),
            FormatoSaida::Terminal => unreachable!(),
        });
    }

    Ok(Configuracao {
        dados,
        formato,
        nivel_ec,
        tamanho_modulo,
        margem,
        caminho_saida,
        pix_config,
    })
}

/// Exibe as instruções de uso e retorna a mensagem de erro
fn exibir_uso() -> String {
    let uso = r#"Gerador de QR Code em Rust

Uso:
  qrcode-generator <texto> [opções]

Opções:
  -f, --formato <tipo>     Formato de saída: terminal, svg, png (padrão: terminal)
  -e, --ec <nível>         Correção de erros: L, M, Q, H (padrão: M)
  -t, --tamanho <pixels>   Tamanho de cada módulo em pixels (padrão: 10)
  -m, --margem <módulos>   Largura da margem/quiet zone (padrão: 4)
  -o, --saida <arquivo>    Caminho do arquivo de saída
  -p, --pix [arquivo]      Gera QR Code PIX (padrão: config_pix.ron)

Exemplos:
  qrcode-generator "Olá, mundo!"
  qrcode-generator "https://rust-lang.org" -f png -o rust.png
  qrcode-generator "Contato" -f svg -e H -t 8
  qrcode-generator "dados" -f png -t 20 -m 2 -o grande.png
  qrcode-generator --pix
  qrcode-generator --pix -f png -o pix.png
  qrcode-generator --pix meu_config.ron -f svg -o pagamento.svg"#;

    eprintln!("{}", uso);
    "Argumentos insuficientes".to_string()
}

fn main() {
    let mut config = match analisar_argumentos() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Erro: {}", e);
            std::process::exit(1);
        }
    };

    // If PIX mode is active, generate the BRCode payload and use it as data
    if let Some(ref pix_path) = config.pix_config {
        let dados_pix = match pix::carregar_config_pix(pix_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Erro: {}", e);
                std::process::exit(1);
            }
        };

        println!("--- Dados PIX ---");
        println!("  Chave:   {}", dados_pix.chave);
        println!("  Nome:    {}", dados_pix.nome);
        println!("  Cidade:  {}", dados_pix.cidade);
        match dados_pix.valor {
            Some(v) => println!("  Valor:   R$ {:.2}", v),
            None => println!("  Valor:   (livre)"),
        }
        println!();

        let payload = match pix::gerar_payload_pix(&dados_pix) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Erro ao gerar payload PIX: {}", e);
                std::process::exit(1);
            }
        };

        println!("--- Payload BRCode ---");
        println!("  {}", payload);
        println!();

        config.dados = payload;
    }

    // Gera o QR Code
    let codigo = match QrCode::with_error_correction_level(&config.dados, config.nivel_ec) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Erro ao gerar QR Code: {}", e);
            eprintln!("Os dados podem ser muito longos para o nível de correção escolhido.");
            std::process::exit(1);
        }
    };

    // Exibe informações sobre o QR Code gerado
    exibir_info(&codigo, &config.dados, config.nivel_ec);

    // Gera a saída no formato solicitado
    let resultado = match config.formato {
        FormatoSaida::Terminal => {
            renderizar_terminal(&codigo);
            Ok(())
        }
        FormatoSaida::Svg => {
            let caminho = config.caminho_saida.as_deref().unwrap_or("qrcode.svg");
            gerar_svg(&codigo, caminho, config.tamanho_modulo, config.margem)
        }
        FormatoSaida::Png => {
            let caminho = config.caminho_saida.as_deref().unwrap_or("qrcode.png");
            gerar_png(&codigo, caminho, config.tamanho_modulo, config.margem)
        }
    };

    if let Err(e) = resultado {
        eprintln!("Erro ao gerar saída: {}", e);
        std::process::exit(1);
    }
}
