# QR Code Generator

Gerador de QR Code em Rust com suporte a múltiplos formatos de saída (Terminal, PNG e SVG).

## Funcionalidades

- **Renderização no terminal** com caracteres Unicode
- **Exportação para PNG** com tamanho configurável
- **Exportação para SVG** vetorial
- **Níveis de correção de erros** configuráveis (L, M, Q, H)
- **Margens e tamanho** de módulos personalizáveis

## Dependências

| Crate   | Versão | Descrição                        |
|---------|--------|----------------------------------|
| `qrcode`| 0.14   | Geração de QR Codes              |
| `image` | 0.25   | Manipulação e exportação de imagens |

## Instalação

```bash
git clone https://github.com/brittos/qrcode-generator.git
cd qrcode-generator
cargo build --release
```

## Uso

### QR Code no terminal (mais rápido para testar)

```bash
cargo run -- "Olá, Mundo!"
```

**Saída esperada:**

```
--- Informações do QR Code ---
  Dados:              Olá, Mundo! (19 caracteres)
  Versão:             2
  Módulos:            25x25
  Correção de erros:  Medium (15%)

[QR Code renderizado com caracteres Unicode]
```

### Gerar PNG de uma URL

```bash
cargo run -- "https://www.rust-lang.org" -f png -o rust_lang.png
```

### Gerar SVG com alta correção de erros

```bash
cargo run -- "Texto importante" -f svg -e H -o importante.svg
```

### Gerar PNG grande com módulos de 20 pixels

```bash
cargo run -- "https://github.com" -f png -t 20 -o github.png
```

### QR Code com margem mínima

```bash
cargo run -- "compacto" -f png -m 1 -o compacto.png
```

## Opções da CLI

```
qrcode-generator <texto> [opções]
```

| Opção                     | Descrição                                          | Padrão     |
|---------------------------|----------------------------------------------------|------------|
| `-f`, `--formato <tipo>`  | Formato de saída: `terminal`, `svg`, `png`         | `terminal` |
| `-e`, `--ec <nível>`      | Correção de erros: `L`, `M`, `Q`, `H`              | `M`        |
| `-t`, `--tamanho <pixels>`| Tamanho de cada módulo em pixels                   | `10`       |
| `-m`, `--margem <módulos>`| Largura da margem/quiet zone                       | `4`        |
| `-o`, `--saida <arquivo>` | Caminho do arquivo de saída                        | —          |

## Licença

Este projeto está sob a licença MIT.
