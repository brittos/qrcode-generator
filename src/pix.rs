use crc_any::CRC;
use serde::Deserialize;
use std::fs;

/// PIX payment data loaded from the RON configuration file.
#[derive(Debug, Deserialize)]
pub struct DadosPix {
    /// PIX key (email, CPF, phone, or random key)
    pub chave: String,
    /// Recipient name (max 25 chars, uppercase, no accents)
    pub nome: String,
    /// Recipient city (max 15 chars, uppercase, no accents)
    pub cidade: String,
    /// Optional fixed transaction amount in BRL
    pub valor: Option<f64>,
}

// ── EMV/BRCode Tag Constants ────────────────────────────────────────────────

/// Payload Format Indicator — always "01"
const TAG_PAYLOAD_FORMAT: &str = "00";
/// Point of Initiation Method — "11" static, "12" dynamic
const TAG_INITIATION_METHOD: &str = "01";
/// Merchant Account Information for PIX (tag 26)
const TAG_MERCHANT_ACCOUNT: &str = "26";
/// Merchant Category Code
const TAG_CATEGORY_CODE: &str = "52";
/// Transaction Currency
const TAG_CURRENCY: &str = "53";
/// Transaction Amount
const TAG_AMOUNT: &str = "54";
/// Country Code
const TAG_COUNTRY: &str = "58";
/// Merchant Name
const TAG_MERCHANT_NAME: &str = "59";
/// Merchant City
const TAG_MERCHANT_CITY: &str = "60";
/// Additional Data Field Template
const TAG_ADDITIONAL_DATA: &str = "62";
/// CRC-16 checksum
const TAG_CRC: &str = "63";

// ── Sub-tag constants ───────────────────────────────────────────────────────

/// GUI sub-tag inside Merchant Account Information
const SUBTAG_GUI: &str = "00";
/// PIX key sub-tag inside Merchant Account Information
const SUBTAG_CHAVE: &str = "01";
/// Reference Label (txid) sub-tag inside Additional Data
const SUBTAG_TXID: &str = "05";

// ── Fixed values per BRCode/BACEN specification ─────────────────────────────

/// PIX GUI identifier registered with BACEN
const PIX_GUI: &str = "br.gov.bcb.pix";
/// ISO 4217 numeric code for Brazilian Real
const CURRENCY_BRL: &str = "986";
/// Generic MCC when category is not applicable
const MCC_GENERIC: &str = "0000";
/// Country code for Brazil
const COUNTRY_BR: &str = "BR";
/// Default txid for static PIX QR Codes
const DEFAULT_TXID: &str = "***";

// ── Core Functions ──────────────────────────────────────────────────────────

/// Builds a single TLV (Tag-Length-Value) field.
///
/// Format: `{tag:2}{length:02}{value}`
///
/// # Example
/// ```
/// assert_eq!(montar_campo_tlv("00", "01"), "000201");
/// ```
fn montar_campo_tlv(tag: &str, valor: &str) -> String {
    format!("{}{:02}{}", tag, valor.len(), valor)
}

/// Builds the Merchant Account Information field (tag 26) for PIX.
///
/// Contains the mandatory GUI (`br.gov.bcb.pix`) and the PIX key.
fn montar_merchant_account(chave: &str) -> String {
    let gui = montar_campo_tlv(SUBTAG_GUI, PIX_GUI);
    let chave_campo = montar_campo_tlv(SUBTAG_CHAVE, chave);
    let conteudo = format!("{}{}", gui, chave_campo);
    montar_campo_tlv(TAG_MERCHANT_ACCOUNT, &conteudo)
}

/// Builds the Additional Data Field Template (tag 62) with the txid.
fn montar_additional_data(txid: &str) -> String {
    let txid_campo = montar_campo_tlv(SUBTAG_TXID, txid);
    montar_campo_tlv(TAG_ADDITIONAL_DATA, &txid_campo)
}

/// Calculates CRC-16/CCITT-FALSE checksum for the PIX payload.
///
/// Uses polynomial 0x1021 with initial value 0xFFFF, as specified
/// by the EMV QR Code standard and BACEN's BRCode manual.
fn calcular_crc16(payload: &str) -> String {
    let mut crc = CRC::crc16ccitt_false();
    crc.digest(payload.as_bytes());
    format!("{:04X}", crc.get_crc() as u16)
}

/// Removes diacritics/accents and truncates text to the specified max length.
///
/// The BRCode specification requires ASCII-only characters for merchant
/// name (max 25) and city (max 15).
fn normalizar_texto(texto: &str, max_len: usize) -> String {
    let normalizado: String = texto
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => {
                if c.is_uppercase() { 'A' } else { 'a' }
            }
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => {
                if c.is_uppercase() {
                    'E'
                } else {
                    'e'
                }
            }
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => {
                if c.is_uppercase() {
                    'I'
                } else {
                    'i'
                }
            }
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => {
                if c.is_uppercase() { 'O' } else { 'o' }
            }
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => {
                if c.is_uppercase() {
                    'U'
                } else {
                    'u'
                }
            }
            'ç' | 'Ç' => {
                if c.is_uppercase() {
                    'C'
                } else {
                    'c'
                }
            }
            'ñ' | 'Ñ' => {
                if c.is_uppercase() {
                    'N'
                } else {
                    'n'
                }
            }
            _ => c,
        })
        .collect();

    let upper = normalizado.to_uppercase();
    if upper.len() <= max_len {
        upper
    } else {
        upper[..max_len].to_string()
    }
}

/// Formats a monetary value with exactly 2 decimal places.
///
/// Removes trailing unnecessary characters while keeping
/// the required precision for BRL amounts.
fn formatar_valor(valor: f64) -> String {
    format!("{:.2}", valor)
}

/// Generates the complete EMV/BRCode payload string for a static PIX QR Code.
///
/// The payload follows the BACEN specification and can be encoded as a
/// QR Code that banking apps will recognize as a valid PIX payment.
///
/// # Errors
///
/// Returns an error if the PIX key is empty.
pub fn gerar_payload_pix(dados: &DadosPix) -> Result<String, String> {
    if dados.chave.is_empty() {
        return Err("A chave PIX não pode estar vazia".to_string());
    }

    let nome = normalizar_texto(&dados.nome, 25);
    let cidade = normalizar_texto(&dados.cidade, 15);

    if nome.is_empty() {
        return Err("O nome do recebedor não pode estar vazio".to_string());
    }
    if cidade.is_empty() {
        return Err("A cidade não pode estar vazia".to_string());
    }

    let mut payload = String::new();

    // Tag 00 — Payload Format Indicator
    payload.push_str(&montar_campo_tlv(TAG_PAYLOAD_FORMAT, "01"));

    // Tag 01 — Point of Initiation Method
    // "11" = static (reusable), "12" = static with fixed value
    let metodo = if dados.valor.is_some() { "12" } else { "11" };
    payload.push_str(&montar_campo_tlv(TAG_INITIATION_METHOD, metodo));

    // Tag 26 — Merchant Account Information (PIX)
    payload.push_str(&montar_merchant_account(&dados.chave));

    // Tag 52 — Merchant Category Code
    payload.push_str(&montar_campo_tlv(TAG_CATEGORY_CODE, MCC_GENERIC));

    // Tag 53 — Transaction Currency (BRL = 986)
    payload.push_str(&montar_campo_tlv(TAG_CURRENCY, CURRENCY_BRL));

    // Tag 54 — Transaction Amount (optional)
    if let Some(valor) = dados.valor
        && valor > 0.0
    {
        payload.push_str(&montar_campo_tlv(TAG_AMOUNT, &formatar_valor(valor)));
    }

    // Tag 58 — Country Code
    payload.push_str(&montar_campo_tlv(TAG_COUNTRY, COUNTRY_BR));

    // Tag 59 — Merchant Name
    payload.push_str(&montar_campo_tlv(TAG_MERCHANT_NAME, &nome));

    // Tag 60 — Merchant City
    payload.push_str(&montar_campo_tlv(TAG_MERCHANT_CITY, &cidade));

    // Tag 62 — Additional Data Field Template (txid)
    payload.push_str(&montar_additional_data(DEFAULT_TXID));

    // Tag 63 — CRC-16 placeholder for calculation
    // The CRC is computed over the entire payload including the "6304" prefix
    payload.push_str(&format!("{}04", TAG_CRC));
    let crc = calcular_crc16(&payload);
    payload.push_str(&crc);

    Ok(payload)
}

/// Loads and deserializes PIX configuration from a RON file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn carregar_config_pix(caminho: &str) -> Result<DadosPix, String> {
    let conteudo =
        fs::read_to_string(caminho).map_err(|e| format!("Erro ao ler '{}': {}", caminho, e))?;

    let dados: DadosPix =
        ron::from_str(&conteudo).map_err(|e| format!("Erro ao parsear '{}': {}", caminho, e))?;

    Ok(dados)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_montar_campo_tlv() {
        assert_eq!(montar_campo_tlv("00", "01"), "000201");
        assert_eq!(montar_campo_tlv("58", "BR"), "5802BR");
        assert_eq!(montar_campo_tlv("53", "986"), "5303986");
    }

    #[test]
    fn test_montar_campo_tlv_tamanho_dois_digitos() {
        let valor = "br.gov.bcb.pix"; // 14 chars
        let resultado = montar_campo_tlv("00", valor);
        assert_eq!(resultado, "0014br.gov.bcb.pix");
    }

    #[test]
    fn test_montar_merchant_account() {
        let resultado = montar_merchant_account("recebedor@email.com");
        // Tag 26, GUI (0014br.gov.bcb.pix) + key (0119recebedor@email.com)
        assert!(resultado.starts_with("26"));
        assert!(resultado.contains("0014br.gov.bcb.pix"));
        assert!(resultado.contains("0119recebedor@email.com"));
    }

    #[test]
    fn test_montar_additional_data() {
        let resultado = montar_additional_data("***");
        assert_eq!(resultado, "62070503***");
    }

    #[test]
    fn test_calcular_crc16() {
        // Known CRC-16 test: the payload ending with "6304" should
        // produce a valid 4-char hex CRC
        let payload = "00020126330014br.gov.bcb.pix0111123456789005204000053039865802BR5913FULANO DE TAL6015BELO HORIZONTE62070503***6304";
        let crc = calcular_crc16(payload);
        assert_eq!(crc.len(), 4);
        // CRC should be uppercase hex
        assert!(crc.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_normalizar_texto_acentos() {
        assert_eq!(normalizar_texto("São Paulo", 15), "SAO PAULO");
        assert_eq!(normalizar_texto("José María", 25), "JOSE MARIA");
        assert_eq!(normalizar_texto("Ação", 10), "ACAO");
    }

    #[test]
    fn test_normalizar_texto_truncamento() {
        assert_eq!(normalizar_texto("BELO HORIZONTE", 15), "BELO HORIZONTE");
        assert_eq!(
            normalizar_texto("CIDADE MUITO LONGA DEMAIS", 15),
            "CIDADE MUITO LO"
        );
    }

    #[test]
    fn test_normalizar_texto_nome_maximo() {
        let nome_longo = "ABCDEFGHIJKLMNOPQRSTUVWXYZ1234"; // 30 chars
        let resultado = normalizar_texto(nome_longo, 25);
        assert_eq!(resultado.len(), 25);
    }

    #[test]
    fn test_formatar_valor() {
        assert_eq!(formatar_valor(150.50), "150.50");
        assert_eq!(formatar_valor(10.0), "10.00");
        assert_eq!(formatar_valor(0.01), "0.01");
        assert_eq!(formatar_valor(1234.56), "1234.56");
    }

    #[test]
    fn test_gerar_payload_sem_valor() {
        let dados = DadosPix {
            chave: "recebedor@email.com".to_string(),
            nome: "FULANO DE TAL".to_string(),
            cidade: "BELO HORIZONTE".to_string(),
            valor: None,
        };

        let payload = gerar_payload_pix(&dados).unwrap();

        // Must start with Payload Format Indicator
        assert!(payload.starts_with("000201"));
        // Static without value → method "11"
        assert!(payload.contains("010211"));
        // Must contain PIX GUI
        assert!(payload.contains("br.gov.bcb.pix"));
        // Must contain the key
        assert!(payload.contains("recebedor@email.com"));
        // Must NOT contain tag 54 (amount)
        assert!(!payload.contains("54"));
        // Must end with CRC (63 04 XXXX)
        assert!(payload.len() > 4);
        let crc_section = &payload[payload.len() - 8..payload.len() - 4];
        assert_eq!(crc_section, "6304");
    }

    #[test]
    fn test_gerar_payload_com_valor() {
        let dados = DadosPix {
            chave: "12345678900".to_string(),
            nome: "FULANO DE TAL".to_string(),
            cidade: "SAO PAULO".to_string(),
            valor: Some(150.50),
        };

        let payload = gerar_payload_pix(&dados).unwrap();

        // Static with value → method "12"
        assert!(payload.contains("010212"));
        // Must contain tag 54 with the formatted amount
        assert!(payload.contains("5406150.50"));
    }

    #[test]
    fn test_gerar_payload_chave_vazia() {
        let dados = DadosPix {
            chave: "".to_string(),
            nome: "FULANO".to_string(),
            cidade: "SP".to_string(),
            valor: None,
        };

        let resultado = gerar_payload_pix(&dados);
        assert!(resultado.is_err());
        assert!(resultado.unwrap_err().contains("chave PIX"));
    }

    #[test]
    fn test_gerar_payload_nome_vazio() {
        let dados = DadosPix {
            chave: "chave@test.com".to_string(),
            nome: "".to_string(),
            cidade: "SP".to_string(),
            valor: None,
        };

        let resultado = gerar_payload_pix(&dados);
        assert!(resultado.is_err());
    }

    #[test]
    fn test_gerar_payload_normaliza_nome_e_cidade() {
        let dados = DadosPix {
            chave: "chave@test.com".to_string(),
            nome: "José da Conceição".to_string(),
            cidade: "São Paulo".to_string(),
            valor: None,
        };

        let payload = gerar_payload_pix(&dados).unwrap();

        // Accents should be removed and text uppercased
        assert!(payload.contains("JOSE DA CONCEICAO"));
        assert!(payload.contains("SAO PAULO"));
    }

    #[test]
    fn test_payload_crc_integrity() {
        // Generate a payload and verify CRC is valid by recalculating
        let dados = DadosPix {
            chave: "test@test.com".to_string(),
            nome: "TESTE".to_string(),
            cidade: "CURITIBA".to_string(),
            valor: Some(10.00),
        };

        let payload = gerar_payload_pix(&dados).unwrap();

        // Extract everything before the CRC value (last 4 chars)
        let sem_crc = &payload[..payload.len() - 4];
        let crc_no_payload = &payload[payload.len() - 4..];
        let crc_recalculado = calcular_crc16(sem_crc);

        assert_eq!(crc_no_payload, crc_recalculado);
    }

    #[test]
    fn test_carregar_config_arquivo_inexistente() {
        let resultado = carregar_config_pix("arquivo_que_nao_existe.ron");
        assert!(resultado.is_err());
    }
}
