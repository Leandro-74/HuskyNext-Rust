# HuskyNext

<p align="center">
  <img src="assets/logo.png" alt="Logo do HuskyNext" width="120">
</p>

<p align="center">
  Controle a iluminação do teclado <b>Husky Sled</b> no Linux com uma interface
  gráfica nativa em Rust e iced — sem depender do software oficial, que é
  exclusivo para Windows.
</p>

> Projeto não oficial, feito por engenharia reversa do protocolo USB HID. Não
> tem vínculo com a Husky. O protocolo pode variar entre modelos e revisões de
> firmware. Use por sua conta e risco.

## O que mudou em relação à versão Python

Esta é uma reescrita completa, com a mesma compatibilidade de protocolo e de
arquivo de configuração, mas com interface gráfica e várias correções:

- Interface iced com seletor de cor (quadrado de saturação/brilho e barra de
  matiz), canais RGB, brilho e os 19 efeitos.
- As alterações são aplicadas automaticamente no teclado. No seletor de cor, o
  envio acontece somente ao soltar o mouse; nos demais controles, o limite é de
  cerca de 8 envios por segundo. O estado final sempre é enviado, e a
  configuração é salva após uma pausa nos ajustes.
- Detecção do controle de iluminação por `usage_page` vendor-specific
  (`0xFF00`–`0xFFFF`), com leitura de report descriptor via sysfs como fallback.
- Reconexão tolerante a configuração antiga: VID/PID continuam valendo mesmo se
  a interface ou o `usage_page` salvos estiverem desatualizados.
- Valores inválidos do JSON são saneados antes de qualquer envio.
- Configuração gravada de forma atômica, com permissões `0600` no Linux.
- O teste de identificação envia **vermelho estático a 100%** de verdade, em vez
  de perguntar se ficou vermelho enquanto enviava branco.
- O dispositivo é redescoberto a cada envio; desconectar e reconectar não exige
  reiniciar o programa.
- Falhas de escrita e de permissão aparecem na interface, com instruções de
  `udev`. O estado só é salvo depois de um envio bem-sucedido.
- A regra `udev` recomendada usa `TAG+="uaccess"` em vez de `MODE="0666"`, que
  liberava o dispositivo para todos os usuários da máquina.
- Modo de diagnóstico por linha de comando (`--list` e `--test`).

## Requisitos

- Linux com Wayland ou X11
- Rust 1.88 ou superior
- Teclado Husky Sled (controladora Sonix `0C45:8501` neste projeto)

O `hidapi` é compilado junto com o backend `hidraw`; não é necessário instalar a
biblioteca do sistema.

## Compilar e executar

```bash
cargo build --release
./target/release/huskynext
```

Para desenvolvimento:

```bash
cargo run
```

### Permissão no Linux

Sem uma regra `udev`, o acesso a `/dev/hidraw*` pode ser negado. Crie
`/etc/udev/rules.d/99-huskynext.rules` com o conteúdo abaixo (ajuste VID/PID se
necessário):

```udev
SUBSYSTEM=="usb", ATTRS{idVendor}=="0c45", ATTRS{idProduct}=="8501", TAG+="uaccess"
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0c45", ATTRS{idProduct}=="8501", TAG+="uaccess"
```

Depois recarregue as regras e reconecte o teclado:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

`TAG+="uaccess"` concede acesso apenas à sessão gráfica ativa. Evite
`MODE="0666"` em máquinas compartilhadas.

## Uso

Na primeira execução, abra a página **Conexão**:

1. Clique em **Detectar novamente**, se necessário.
2. Use **Testar** para enviar vermelho estático a 100%. Se a luz mudar, o
   dispositivo correto é aquele.
3. Clique em **Usar este dispositivo** para salvar VID, PID, interface e
   `usage_page` em `~/.huskynext/config.json`.

Na página **Iluminação**:

- **Cor**: arraste no quadrado de saturação/brilho, escolha a matiz na barra
  inferior, digite um HEX (`#FF5733` ou `FF5733`), use os canais R/G/B ou os
  atalhos de cor.
- **Brilho**: cinco níveis de 0% a 100%.
- **Efeitos**: os 19 modos do protocolo.
- **Aplicação automática**: no quadrado/barra de matiz, a cor só é enviada
  quando você solta o mouse. Brilho, efeitos, HEX e canais RGB usam um limite de
  aproximadamente 8 envios por segundo. O estado final sempre é enviado; se algo
  falhar, a interface oferece **Tentar novamente**.

A configuração é compatível com o formato usado pela versão Python, então quem
já usava o projeto anterior mantém o mesmo arquivo.

## Linha de comando

Útil para diagnosticar permissões e confirmar o protocolo sem abrir a GUI:

```bash
huskynext --list          # lista os dispositivos HID detectados
huskynext --test 00FF88   # envia cor estática a 100% (padrão: FF0000)
huskynext --help
```

`--test` usa o dispositivo salvo na configuração; se não houver configuração,
procura a controladora Sonix ou o primeiro dispositivo vendor-specific.

## Protocolo

O resumo completo está em [PROTOCOL.md](PROTOCOL.md). Em resumo:

| Item | Valor |
| --- | --- |
| Tipo | HID output report (`SET_REPORT`) |
| Report ID | `0x04` |
| Tamanho | 64 bytes (ID + 63 de payload) |
| Modo | offset `0x09`, `0x00`–`0x12` |
| Brilho | offset `0x0A`, `0x00`–`0x04` |
| Cor | offsets `0x0E`, `0x0F`, `0x10` (R, G, B) |

Cada envio carrega o estado completo. O teclado não responde nem fornece leitura
de estado neste protocolo.

## Estrutura

```
src/
├── main.rs       # entrada, argumentos de CLI e diagnóstico
├── app.rs        # interface iced, navegação e fluxo de configuração
├── config.rs     # leitura/gravação atômica de ~/.huskynext/config.json
├── device.rs     # enumeração HID, descoberta e escrita do report
└── protocol.rs   # efeitos, brilho, cor, montagem e testes do report
```

A pasta local `HuskyNext/` contém a implementação Python original, mantida como
referência histórica do protocolo. Ela é um checkout separado e está ignorada
por este repositório.

## Qualidade

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

Os testes cobrem a montagem do relatório, parsing de cor, sanitização da
configuração, gravação atômica, parsing de report descriptors e a seleção de
dispositivo mesmo com configuração antiga.

## Limitações

- O protocolo é somente de escrita; não há como ler o estado atual do teclado.
- Não há suporte a perfis, macros ou efeitos customizados.
- Outros modelos de teclado podem usar offsets ou IDs diferentes.
- Em Wayland, a janela é controlada pelo compositor; tamanho e posição iniciais
  podem variar.
