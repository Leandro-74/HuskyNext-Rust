<p align="center">
  <img src="assets/logo.png" alt="Logo do HuskyNext" width="120">
</p>

<h1 align="center">HuskyNext</h1>

<p align="center">
  Controle a iluminação do teclado <b>Husky Sled</b> no Linux com uma interface nativa em Rust e iced, sem o software oficial, que é exclusivo para Windows.
</p>

<p align="center">
  <a href="https://github.com/Leandro-74/HuskyNext-Rust">English</a> | <a href="README.pt-BR.md">Português</a>
</p>

<p align="center">
  <a href="https://github.com/Leandro-74/HuskyNext-Rust/actions/workflows/ci.yml"><img src="https://github.com/Leandro-74/HuskyNext-Rust/actions/workflows/ci.yml/badge.svg" alt="Status da CI"></a>
</p>

> ⚠️ Projeto não oficial. O protocolo foi obtido por engenharia reversa do tráfego USB HID. Ele não tem vínculo com a Husky. O protocolo pode variar entre modelos e revisões de firmware. Use por sua conta e risco.

## Sobre

HuskyNext é uma implementação independente e autocontida, em Rust, do controle de iluminação do teclado Husky Sled. Ela conversa diretamente com a interface HID vendor-specific e roda como um aplicativo desktop nativo em Wayland ou X11.

A interface é construída com iced. As configurações ficam em um pequeno arquivo JSON, e cada alteração é enviada ao teclado como um relatório de saída completo de 64 bytes.

## Recursos

- Controle a cor, o brilho, a velocidade e os 19 efeitos de iluminação.
- Escolha cores com um quadrado de saturação/brilho, uma barra de matiz, entrada HEX, canais RGB ou presets.
- Aplique alterações automaticamente a até cerca de 8 envios por segundo.
- Envie a cor do seletor somente ao soltar o mouse.
- Salve a configuração após uma breve pausa nos ajustes.
- Detecte a interface de iluminação por Usage Page vendor-specific, com fallback pelo report descriptor do sysfs.
- Armazene em cache os dispositivos descobertos e redescubra apenas quando um path ficar obsoleto ou ao detectar novamente.
- Recupere a conexão após desconectar e reconectar o teclado, sem reiniciar o aplicativo.
- Identifique o dispositivo com uma luz roxa estática real (#8835E9) a 100%.
- Esqueça um dispositivo sem perder o estado de iluminação; o próximo dispositivo sobrescreve os campos antigos.
- Leia arquivos de configuração JSON antigos no formato plano e saneie valores inválidos antes de enviar.
- Grave a configuração de forma atômica, com permissões restritas ao usuário.
- Diagnostique o dispositivo pela linha de comando com `--list` e `--test`.
- Conceda acesso com uma regra udev que usa `TAG+="uaccess"` em vez de abrir o dispositivo para todos os usuários.

## Requisitos

- Linux com Wayland ou X11.
- Rust 1.88 ou superior.
- Teclado Husky Sled (controladora Sonix `0C45:8501` neste projeto).
- `pkg-config` ou `pkgconf`.
- Headers de desenvolvimento do libudev: `libudev-dev` no Debian e Ubuntu, `systemd-libs` no Arch Linux.

O `hidapi` é compilado com o backend `hidraw`. A biblioteca `hidapi` do sistema não é necessária, mas os headers do libudev são necessários no build.

## Instalação

```bash
git clone https://github.com/Leandro-74/HuskyNext-Rust.git
cd HuskyNext-Rust
cargo build --release
sudo install -Dm755 target/release/huskynext /usr/local/bin/huskynext
```

Para executar sem instalar, use `./target/release/huskynext`.

### Regra udev

Sem uma regra udev, o acesso a `/dev/hidraw*` pode ser negado. Instale a regra do repositório:

```bash
sudo install -Dm644 packaging/99-huskynext.rules /etc/udev/rules.d/99-huskynext.rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Reconecte o teclado depois de carregar a regra.

A regra usa os IDs Sonix `0c45:8501`. Ajuste os IDs se o seu teclado usar valores diferentes. `TAG+="uaccess"` concede acesso apenas ao usuário da sessão local ativa. Não use `MODE="0666"` em máquinas compartilhadas, porque isso permite que todo usuário local grave no dispositivo.

### Atalho no menu

```bash
install -Dm644 packaging/huskynext.desktop ~/.local/share/applications/huskynext.desktop
update-desktop-database ~/.local/share/applications
```

Para um atalho de sistema, instale o mesmo arquivo em `/usr/share/applications` com `sudo`. O atalho usa `Exec=huskynext`, então o binário precisa estar no `PATH`.

### Ícone

`assets/logo.png` é uma imagem de 512×512. Instale-a no diretório correspondente:

```bash
install -Dm644 assets/logo.png ~/.local/share/icons/hicolor/512x512/apps/huskynext.png
gtk-update-icon-cache -f -t ~/.local/share/icons/hicolor
```

Para um ícone de sistema, use `/usr/share/icons/hicolor/512x512/apps/huskynext.png` com `sudo` e atualize o cache nesse diretório. Se precisar de outro tamanho, redimensione a imagem antes de copiar.

## Uso

```bash
huskynext
```

O aplicativo abre com três páginas: **Lighting**, **Connection** e **About**. A barra lateral mostra o estado atual da conexão.

### Primeira execução

1. Conecte o teclado.
2. Inicie o aplicativo. Se nenhum teclado estiver configurado, a página **Connection** abre primeiro.
3. Clique em **Detect again** se a lista estiver vazia.
4. Clique em **Test** em um dispositivo. O teclado deve ficar roxo (`#8835E9`) a 100%.
5. Clique em **Use this device** para salvar o vendor ID, o product ID, a interface e a Usage Page.
6. Abra a página **Lighting** e ajuste a cor, o brilho, a velocidade ou o efeito.

A configuração fica em `~/.huskynext/config.json`. Para redefini-la, feche o aplicativo e remova esse arquivo. **Forget device** remove apenas o vínculo com o dispositivo e mantém as configurações de iluminação.

### Opções

- `huskynext`: abre a interface gráfica.
- `huskynext --list`: lista os dispositivos HID detectados com IDs, interface, Usage Page e path.
- `huskynext --test [HEX]`: envia uma cor estática a 100% para o dispositivo salvo. A cor padrão é `8835E9`.
- `huskynext --version`: mostra a versão.
- `huskynext --help`: mostra o texto de ajuda.

`--test` usa o dispositivo salvo na configuração. Se o dispositivo salvo não for encontrado, o comando falha e não envia a cor para outro dispositivo. Sem dispositivo salvo, ele prefere a controladora Sonix ou Sled e depois o primeiro dispositivo vendor-specific. Uma cor inválida retorna o código de saída 2. Um aviso sobre arquivo de configuração inválido é impresso na saída de erro padrão.

As opções seguem esta prioridade: `--help`, `--version`, `--list`, `--test`.

## Estrutura do projeto

```text
HuskyNext-Rust/
├── src/
│   ├── app.rs       # interface iced, estado do aplicativo e aplicação automática
│   ├── color.rs     # conversão entre HSV e RGB
│   ├── config.rs    # leitura e gravação de ~/.huskynext/config.json
│   ├── device.rs    # enumeração HID, descoberta, seleção e escrita
│   ├── lib.rs       # raiz da biblioteca
│   ├── main.rs      # entrada, linha de comando e diagnóstico
│   └── protocol.rs  # efeitos, brilho, velocidade, cor e layout do relatório
├── assets/
│   └── logo.png
├── packaging/
│   ├── 99-huskynext.rules
│   └── huskynext.desktop
├── .github/workflows/ci.yml
├── Cargo.toml
├── Cargo.lock
├── CHANGELOG.md
├── LICENSE
├── PROTOCOL.md
├── README.md
└── README.pt-BR.md
```

## Como funciona

### Descoberta de dispositivos

O aplicativo enumera os dispositivos HID com `hidapi`. Um dispositivo é tratado como candidato de iluminação quando a Usage Page está no intervalo vendor-specific `0xFF00`–`0xFFFF`. No Linux, o aplicativo também lê os report descriptors em `/sys/class/hidraw/*/device/report_descriptor` e procura uma Usage Page vendor-specific no descritor.

Entradas duplicadas são agrupadas por path, interface e IDs. Dispositivos vendor-specific aparecem primeiro na lista.

Ao conectar, o aplicativo filtra os candidatos por vendor ID e product ID. A interface e a Usage Page salvas são usadas apenas quando ainda correspondem a um candidato. Isso mantém uma configuração antiga utilizável depois que uma atualização de firmware muda esses valores.

A lista de dispositivos fica em cache. Uma nova varredura ocorre apenas quando um path fica obsoleto, quando o usuário clica em **Detect again** ou durante o fluxo de conexão manual. Se um path em cache estiver obsoleto, o aplicativo redescobre os dispositivos uma vez e repete o envio. Erros de permissão não iniciam esse ciclo.

### Aplicação automática

Alterações marcam o estado como pendente. Um temporizador roda a cada 120 ms, o que dá cerca de 8 envios por segundo. O seletor de cor é a exceção: a cor é enviada ao soltar o mouse, não a cada movimento.

Depois da última alteração, a configuração é salva após uma pausa de cerca de 450 ms. Um **Apply** manual envia o estado e salva o arquivo imediatamente.

Se um envio automático falhar, a aplicação automática pausa e a interface mostra o erro. O estado final é enviado quando o usuário clica em **Try again**. A configuração só é salva depois de um envio bem-sucedido.

### Configuração

O arquivo `~/.huskynext/config.json` é um objeto JSON plano. O aplicativo lê arquivos com campos ausentes e valores antigos. Valores inválidos são saneados antes de qualquer envio: um efeito desconhecido vira **Static**, o brilho é limitado a 0–100% e a velocidade é limitada aos níveis 1–5.

Campos vazios são omitidos ao gravar. Uma configuração sem dispositivo e sem valores de iluminação é gravada como `{}`. O arquivo é gravado de forma atômica e recebe permissões `0600` no Linux.

**Forget device** limpa o vendor ID, o product ID, a interface e a Usage Page. O estado de iluminação é preservado. Um novo dispositivo sobrescreve os quatro campos de vínculo, mesmo quando a interface ou a Usage Page estão ausentes. Depois de **Forget device**, o arquivo contém apenas o estado de iluminação, e a compatibilidade de ida e volta com ferramentas antigas não é garantida.

### Protocolo

A descrição completa está em [PROTOCOL.md](PROTOCOL.md). Em resumo:

| Item | Valor |
| --- | --- |
| Tipo | HID output report (`SET_REPORT`) |
| Report ID | `0x04` |
| Tamanho | 64 bytes (ID mais 63 bytes de payload) |
| Efeito | offset `0x09`, `0x00`–`0x12` |
| Brilho | offset `0x0A`, `0x00`–`0x04` |
| Velocidade | offset `0x0B`, `0x04`–`0x00` (1=lenta, 5=rápida) |
| Cor | offsets `0x0E`, `0x0F` e `0x10` (R, G, B) |

Cada relatório carrega o estado completo. O teclado não responde e não fornece leitura de estado neste protocolo.

## Qualidade

O workflow de integração contínua roda em cada push para a `main` e em pull requests. Ele usa Rust 1.88.0 e stable, instala o `libudev-dev` e executa:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo build --release
```

Rode as mesmas verificações localmente antes de enviar alterações. A suíte tem 62 testes: 55 na biblioteca e 7 no binário. Os testes cobrem a montagem do relatório, o parsing de cor, a sanitização e a gravação atômica da configuração, o parsing de report descriptors, a seleção de dispositivo com configurações antigas, a detecção de path obsoleto e a validação do relatório.

## Solução de problemas

- **Permissão negada ao abrir o dispositivo**: instale a regra udev de `packaging/99-huskynext.rules`, recarregue as regras e reconecte o teclado. A interface mostra a regra esperada com os IDs do dispositivo detectado para erros de permissão.
- **Teclado configurado não encontrado**: rode `huskynext --list`, verifique a conexão USB e clique em **Detect again**. `--test` falha nesse caso; ele não envia para outro dispositivo.
- **Aviso de configuração inválida**: o arquivo foi ignorado e os padrões foram usados. Corrija o JSON ou remova `~/.huskynext/config.json`.
- **O teclado continua roxo depois de um teste**: essa é a cor de identificação. Clique em **Use this device** e depois escolha a iluminação desejada.
- **O build falha por falta do libudev**: instale os headers de desenvolvimento. Use `libudev-dev` no Debian ou Ubuntu e `systemd-libs` no Arch Linux.
- **O ícone não aparece**: confirme o nome `huskynext.png` e o diretório `512x512` e atualize o cache de ícones.
- **Tamanho da janela no Wayland**: o compositor controla o tamanho e a posição iniciais. Mova ou redimensione a janela com as ferramentas do compositor.

## Contribuição

Encontrou um bug ou tem uma ideia? Abra uma [issue](https://github.com/Leandro-74/HuskyNext-Rust/issues) ou envie um pull request.

Mantenha os textos públicos em inglês e rode as verificações de qualidade antes de enviar uma alteração.

## Licença

Este projeto usa a licença MIT. Veja o arquivo [LICENSE](LICENSE) para detalhes.
