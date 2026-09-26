//! Ícone do **Git** desenhado EM CÓDIGO.
//!
//! **O quê:** gera o PNG do app em qualquer tamanho, sem asset externo e sem rasterizador.
//!
//! **Onde:** [`super::desktop::instalar`], ao gravar a entrada no menu.
//!
//! ## Mesma família, forma própria — e aqui a distinção custou pensamento
//!
//! Os apps da casa usam o mesmo squircle e o mesmo fundo, e se distinguem por cor e forma:
//!
//! - schematize: azul, grafo de 3 nós em triângulo.
//! - deployer: verde-água, 3 nós em CADEIA diagonal (origem → ponte → destino).
//! - database: âmbar, faixas empilhadas com divisória de coluna (uma tabela).
//! - git: violeta, o TRONCO VERTICAL com uma ramificação (um branch).
//!
//! **O risco aqui era virar o ícone do deployer.** Os dois são nós ligados por arestas, e a
//! distinção não pode depender só da cor — ela é a primeira coisa que se perde em 16px, em
//! tema claro e para quem não distingue matiz. O que separa os dois é a GEOMETRIA: a do
//! deployer é uma diagonal contínua, a daqui é uma vertical com uma saída em ângulo reto.
//! Diagonal e ortogonal se distinguem de longe, mesmo em cinza.

// Paleta: o MESMO fundo da casa, acento próprio.
const BG_TL: [f32; 3] = [0x14 as f32, 0x16 as f32, 0x1c as f32]; // #14161c (canto sup-esq)
const BG_BR: [f32; 3] = [0x1b as f32, 0x1e as f32, 0x27 as f32]; // #1b1e27 (canto inf-dir)
const ACCENT: [f32; 3] = [0x9b as f32, 0x7c as f32, 0xf0 as f32]; // #9b7cf0 violeta (Git)
const ACCENT_HI: [f32; 3] = [0xc7 as f32, 0xb2 as f32, 0xff as f32]; // #c7b2ff (a ponta do ramo)
const RING: [f32; 3] = [0x14 as f32, 0x16 as f32, 0x1c as f32]; // anel escuro separando nó da aresta

/// Amostras por eixo no supersampling (SS×SS por pixel) → bordas suaves sem lib de imagem.
const SS: u32 = 4;

/// Os nós, em FRAÇÃO do lado: topo do tronco, base do tronco, ponta do ramo.
///
/// **A ponta do ramo é a CLARA**, porque é a informação: o tronco é o que sempre existe, o ramo
/// é o que mudou. Dar o destaque ao tronco seria destacar o que não diz nada.
const NOS: [(f32, f32, [f32; 3]); 3] = [
    (0.360, 0.235, ACCENT),    // topo do tronco
    (0.360, 0.780, ACCENT),    // base do tronco
    (0.690, 0.760, ACCENT_HI), // a ponta do ramo
];

/// Os pontos do traçado, em FRAÇÃO: tronco vertical + ramo em ângulo reto.
///
/// **Ângulo reto, e não uma diagonal**, para não virar o ícone do deployer — que é uma cadeia
/// diagonal de três nós. A diferença entre ortogonal e diagonal sobrevive ao cinza e aos 16px;
/// a diferença entre verde-água e violeta, não.
const ARESTAS: [((f32, f32), (f32, f32)); 3] = [
    ((0.360, 0.235), (0.360, 0.780)), // o tronco
    ((0.360, 0.505), (0.690, 0.505)), // a saída do ramo
    ((0.690, 0.505), (0.690, 0.760)), // a descida até a ponta
];

/// **O quê:** o ícone em RGBA no tamanho `n` (px). Devolve `(bytes, w, h)`. PURO e determinístico.
///
/// **Onde:** [`write_png`], e os testes — que podem afirmar pixels sem tocar no disco.
pub fn rgba(n: u32) -> (Vec<u8>, u32, u32) {
    let nf = n as f32;
    let radius = nf * 0.227; // rx=232/1024, o mesmo squircle da casa
    let node_r = nf * 0.082;
    let ring_w = nf * 0.018;
    let edge_w = nf * 0.047;

    let mut buf = vec![0u8; (n * n * 4) as usize];
    let inv_ss2 = 1.0 / (SS * SS) as f32;

    for y in 0..n {
        for x in 0..n {
            let (mut r, mut g, mut b, mut a) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
            for sy in 0..SS {
                for sx in 0..SS {
                    let px = x as f32 + (sx as f32 + 0.5) / SS as f32;
                    let py = y as f32 + (sy as f32 + 0.5) / SS as f32;
                    if let Some(c) = sample(px, py, nf, radius, node_r, ring_w, edge_w) {
                        r += c[0];
                        g += c[1];
                        b += c[2];
                        a += 255.0;
                    }
                }
            }
            let idx = ((y * n + x) * 4) as usize;
            let cover = a * inv_ss2;
            if cover > 0.0 {
                // Média só sobre os subpixels COBERTOS: dividir pelo total criaria um halo
                // escuro na borda, porque os vazios entrariam como preto.
                let covered = a / 255.0;
                buf[idx] = (r / covered).round().clamp(0.0, 255.0) as u8;
                buf[idx + 1] = (g / covered).round().clamp(0.0, 255.0) as u8;
                buf[idx + 2] = (b / covered).round().clamp(0.0, 255.0) as u8;
                buf[idx + 3] = cover.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    (buf, n, n)
}

/// **O quê:** a cor OPACA de um subpixel, ou `None` fora do squircle (transparente).
///
/// **Onde:** [`rgba`]. Ordem: fundo (gradiente) → arestas → anel do nó → miolo do nó.
fn sample(
    px: f32,
    py: f32,
    nf: f32,
    radius: f32,
    node_r: f32,
    ring_w: f32,
    edge_w: f32,
) -> Option<[f32; 3]> {
    if !inside_rounded(px, py, nf, radius) {
        return None;
    }
    let t = ((px + py) / (2.0 * nf)).clamp(0.0, 1.0);
    let mut color = [
        BG_TL[0] + (BG_BR[0] - BG_TL[0]) * t,
        BG_TL[1] + (BG_BR[1] - BG_TL[1]) * t,
        BG_TL[2] + (BG_BR[2] - BG_TL[2]) * t,
    ];
    for (a, b) in ARESTAS {
        if dist_ao_segmento(px, py, (a.0 * nf, a.1 * nf), (b.0 * nf, b.1 * nf)) <= edge_w * 0.5 {
            color = ACCENT;
            break;
        }
    }
    // O anel escuro separa o nó da aresta — sem ele o traçado e o nó viram uma mancha só.
    for (cx, cy, cor) in NOS {
        let (dx, dy) = (px - cx * nf, py - cy * nf);
        let d = (dx * dx + dy * dy).sqrt();
        if d <= node_r + ring_w {
            color = if d <= node_r { cor } else { RING };
            break;
        }
    }
    Some(color)
}

/// **O quê:** ponto dentro de um quadrado `0..n` com cantos de raio `r`.
fn inside_rounded(px: f32, py: f32, n: f32, r: f32) -> bool {
    if px < 0.0 || py < 0.0 || px > n || py > n {
        return false;
    }
    let cx = px.clamp(r, n - r);
    let cy = py.clamp(r, n - r);
    let (dx, dy) = (px - cx, py - cy);
    dx * dx + dy * dy <= r * r
}

/// **O quê:** distância de um ponto ao segmento `p1`–`p2`.
fn dist_ao_segmento(px: f32, py: f32, p1: (f32, f32), p2: (f32, f32)) -> f32 {
    let (x1, y1) = p1;
    let (x2, y2) = p2;
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }
    let t = (((px - x1) * dx + (py - y1) * dy) / len2).clamp(0.0, 1.0);
    ((px - (x1 + t * dx)).powi(2) + (py - (y1 + t * dy)).powi(2)).sqrt()
}

/// **O quê:** escreve o ícone (tamanho `n`) como PNG em `path`, criando os diretórios-pai.
pub fn write_png(path: &std::path::Path, n: u32) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let (data, w, h) = rgba(n);
    let file = std::fs::File::create(path)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(std::io::Error::other)?;
    writer.write_image_data(&data).map_err(std::io::Error::other)?;
    Ok(())
}

/// Nome do arquivo de ícone. Um lugar só: se divergir do `Icon=` do `.desktop`, o menu mostra
/// um quadrado cinza e ninguém liga a causa ao nome.
pub const NOME: &str = "schematize-git";

/// Tamanhos hicolor padrão (freedesktop).
pub const HICOLOR_SIZES: [u32; 8] = [16, 24, 32, 48, 64, 128, 256, 512];

/// **O quê:** gera a árvore hicolor completa em `base`. Devolve os caminhos escritos.
pub fn write_hicolor(base: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
    let mut out = Vec::new();
    for n in HICOLOR_SIZES {
        let p = base.join(format!("{n}x{n}")).join("apps").join(format!("{NOME}.png"));
        write_png(&p, n)?;
        out.push(p);
    }
    Ok(out)
}

/// **O quê:** instala o ícone em todos os locais que os ambientes consultam, e devolve o
/// caminho ABSOLUTO do 256px — o que vai no `Icon=`.
///
/// **Absoluto no `Icon=` de propósito:** no Wayland o dock casa a janela ao `.desktop`, e um
/// nome de tema depende de cache de ícones que pode estar velho ou quebrado.
pub fn install_all(home: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    let icons = home.join(".local/share/icons/hicolor");
    write_hicolor(&icons)?;
    let p256 = icons.join("256x256").join("apps").join(format!("{NOME}.png"));
    for extra in [
        home.join(format!(".local/share/pixmaps/{NOME}.png")),
        home.join(format!(".icons/{NOME}.png")),
        home.join(format!(".local/share/icons/{NOME}.png")),
    ] {
        let _ = write_png(&extra, 256);
    }
    Ok(p256)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cor(buf: &[u8], n: u32, fx: f32, fy: f32) -> [u8; 3] {
        let (x, y) = ((fx * n as f32) as u32, (fy * n as f32) as u32);
        let i = ((y * n + x) * 4) as usize;
        [buf[i], buf[i + 1], buf[i + 2]]
    }

    /// O canto é TRANSPARENTE e o centro do tronco é OPACO — é o squircle existindo.
    #[test]
    fn o_squircle_recorta_os_cantos() {
        let (buf, n, _) = rgba(64);
        let alpha = |x: u32, y: u32| buf[((y * n + x) * 4 + 3) as usize];
        assert_eq!(alpha(0, 0), 0, "o canto tem de ser transparente");
        assert_eq!(alpha(63, 63), 0);
        assert_eq!(alpha(32, 32), 255, "o miolo tem de ser opaco");
    }

    /// **O TRONCO é vertical e o RAMO sai em ângulo reto.**
    ///
    /// É o que separa este ícone do deployer, que é uma cadeia DIAGONAL de três nós. A
    /// distinção tem de sobreviver ao cinza e aos 16px, e a cor não sobrevive a nenhum dos dois.
    #[test]
    fn o_tronco_e_vertical_e_o_ramo_sai_em_angulo_reto() {
        let n = 512u32;
        let (buf, _, _) = rgba(n);
        // **A comparação é com o ACENTO, e não com "a cor do fundo em outro ponto".**
        //
        // O fundo é um GRADIENTE diagonal: dois pontos distintos têm cores distintas, e um
        // `assert_eq!(aqui, fundo_de_lá)` reprovaria sobre pixels corretos. A primeira versão
        // deste teste fazia isso e reprovou — a afirmação certa é sobre o traçado existir ou
        // não, que é o que o ícone comunica.
        let acento = [ACCENT[0] as u8, ACCENT[1] as u8, ACCENT[2] as u8];

        // O tronco existe em três alturas, sempre no MESMO x: é o que "vertical" quer dizer.
        //
        // As alturas fogem dos NÓS de propósito. Um nó tem anel escuro de raio `node_r+ring_w`
        // (0.10 do lado), e `y=0.33` cai dentro do anel do nó de topo — o pixel lá é o anel, e
        // não o traçado. A primeira versão deste teste reprovou exatamente aí, afirmando sobre
        // um pixel correto.
        for fy in [0.40, 0.50, 0.62] {
            assert_eq!(cor(&buf, n, 0.360, fy), acento, "o tronco sumiu em y={fy}");
        }
        // E a saída do ramo é horizontal: o mesmo y, x diferentes.
        for fx in [0.45, 0.55, 0.65] {
            assert_eq!(cor(&buf, n, fx, 0.505), acento, "a saída do ramo sumiu em x={fx}");
        }
        // **Nada na diagonal entre o tronco e a ponta.** É aqui que ele viraria o deployer:
        // uma reta de (0.36,0.24) a (0.69,0.76) passaria por estes pontos.
        for (fx, fy) in [(0.46_f32, 0.38_f32), (0.52, 0.63), (0.60, 0.68)] {
            assert_ne!(
                cor(&buf, n, fx, fy),
                acento,
                "traço diagonal em ({fx},{fy}) — é o deployer"
            );
        }
    }

    /// **A ponta do ramo é a mais CLARA.** O tronco é o que sempre existe; o ramo é o que
    /// mudou. Destacar o tronco seria destacar o que não diz nada.
    #[test]
    fn a_ponta_do_ramo_e_o_destaque() {
        let n = 512u32;
        let (buf, _, _) = rgba(n);
        let ponta = cor(&buf, n, NOS[2].0, NOS[2].1);
        let tronco = cor(&buf, n, NOS[0].0, NOS[0].1);
        assert_ne!(ponta, tronco);
        assert!(
            ponta[0] > tronco[0] && ponta[1] > tronco[1] && ponta[2] > tronco[2],
            "a ponta tem de ser a mais clara: {ponta:?} vs {tronco:?}"
        );
    }

    /// Todo tamanho hicolor sai, e sai com o número de bytes certo.
    #[test]
    fn sai_em_todo_tamanho_do_hicolor() {
        for n in HICOLOR_SIZES {
            let (buf, w, h) = rgba(n);
            assert_eq!((w, h), (n, n));
            assert_eq!(buf.len(), (n * n * 4) as usize, "tamanho {n}");
        }
    }
}
