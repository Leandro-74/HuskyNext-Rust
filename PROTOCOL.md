# Protocolo HuskyNext

Documento técnico do protocolo USB HID usado pelo HuskyNext para controlar a
iluminação do teclado **Husky Sled**. As informações abaixo foram obtidas por
engenharia reversa (captura com Wireshark/USBPcap do software oficial) e
correspondem ao que está implementado em `src/`.

> ⚠️ Protocolo não-oficial, sem vínculo com a Husky. Pode variar entre modelos e
> revisões de firmware.

## 1. Visão geral

| Item | Valor |
| --- | --- |
| Barramento | USB HID |
| Direção | Host → dispositivo (apenas escrita) |
| Tipo de relatório | Output (`SET_REPORT`) |
| Report ID | `0x04` |
| Tamanho do relatório | 64 bytes (incluindo o Report ID) |
| Byte order | Little-endian |
| Estado | Cada relatório carrega o estado completo (modo + brilho + velocidade + RGB) |

A comunicação é feita com a biblioteca `hidapi` (`hid.device.write()`), sem
leitura de respostas (o dispositivo não envia `GET_REPORT`/input reports neste
protocolo). Ver `src/device.py:126`.

## 2. Identificação do dispositivo

O teclado expõe várias interfaces/collections HID. A interface de controle de
iluminação é identificada por um **Usage Page vendor-specific** no intervalo
`0xFF00`–`0xFFFF` (`src/device.py:6`).

A descoberta funciona assim (`src/device.py:67`):

1. Enumera os dispositivos HID com `hid.enumerate()`.
2. Considera candidato qualquer dispositivo cujo `usage_page` esteja em
   `0xFF00`–`0xFFFF`.
3. No Linux, complementa a detecção lendo os report descriptors em
   `/sys/class/hidraw/*/device/report_descriptor` e procurando um item
   `Usage Page` vendor-specific no descritor (`src/device.py:16`).
4. Remove duplicatas pela chave `(vendor_id, product_id, interface_number)`.
5. Se nenhum candidato vendor-specific existir, usa a lista completa de
   dispositivos como fallback.

O descritor é interpretado item a item: itens longos (`0xFE`) são pulados e o
item de tag `0x04` (`Usage Page`) tem seu valor lido em little-endian.

### Dados persistidos

Após confirmação no assistente, a conexão é salva em
`~/.huskynext/config.json`:

```json
{
  "vendor_id": 3141,
  "product_id": 34049,
  "interface_number": 1,
  "usage_page": 65280,
  "effect": 6,
  "bright": 4,
  "speed": 4,
  "r": 255,
  "g": 255,
  "b": 255
}
```

A abertura do dispositivo filtra por `vendor_id` + `product_id` +
`interface_number` e, se disponível, por `usage_page`
(`src/device.py:93`).

## 3. Formato do relatório

O relatório tem 64 bytes. O primeiro byte é o Report ID (`0x04`) e os demais
formam o payload. O pacote base está em `src/colors.py:6`:

```
offset  00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
        04 A1 01 06 38 00 00 55 00 06 04 01 00 00 FF FF
offset  10 11 12 13 14 15 16 17 18 19 1A 1B 1C 1D 1E 1F
        FF 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00
offset  20 ... 3F
        00 ... 00
```

### Campos variáveis

| Offset | Campo | Valores | Padrão |
| --- | --- | --- | --- |
| `0x00` | Report ID | `0x04` fixo | `0x04` |
| `0x09` | Modo/efeito | `0x00`–`0x12` (ver tabela) | `0x06` (Estático) |
| `0x0A` | Brilho | `0x00`–`0x04` (0–100%) | `0x04` (100%) |
| `0x0B` | Velocidade | `0x00`–`0x04` (5=rápida … 1=lenta) | `0x01` (nível 4) |
| `0x0E` | Canal R | `0x00`–`0xFF` | `0xFF` |
| `0x0F` | Canal G | `0x00`–`0xFF` | `0xFF` |
| `0x10` | Canal B | `0x00`–`0xFF` | `0xFF` |

Constantes correspondentes em `src/colors.py:17` (o offset de velocidade foi
identificado depois, por teste no teclado, e não existe na versão Python):

```python
MODE_OFFSET     = 9   # 0x09
BRIGHT_OFFSET   = 10  # 0x0A
COLOR_OFFSET_R  = 14  # 0x0E
COLOR_OFFSET_G  = 15  # 0x0F
COLOR_OFFSET_B  = 16  # 0x10
```

Na implementação Rust, `SPEED_OFFSET = 11` (`0x0B`).

Os demais bytes (`0x01`–`0x08`, `0x0C`–`0x0D`, `0x11`–`0x3F`) permanecem
constantes no protocolo implementado:

```
A1 01 06 38 00 00 55 00 00 <modo> <brilho> <velocidade> 00 00 <R> <G> <B> 00...
```

## 4. Modos de iluminação

Valores aceitos no offset `0x09` (`src/colors.py:23`):

| Código | Modo |
| --- | --- |
| `0x00` | Desligado |
| `0x01` | Onda |
| `0x02` | Persianas |
| `0x03` | Difusão |
| `0x04` | Néon |
| `0x05` | Respiração |
| `0x06` | Estático |
| `0x07` | Reativo |
| `0x08` | Gota de Chuva |
| `0x09` | Reativa por Linha |
| `0x0A` | Iluminado por Estrelas |
| `0x0B` | Flores em Flor |
| `0x0C` | Cascata |
| `0x0D` | Arco-íris |
| `0x0E` | Desenho Animado |
| `0x0F` | Cena de Chuva |
| `0x10` | Escaneamento |
| `0x11` | Single Off |
| `0x12` | Fluxo |

## 5. Brilho

O brilho usa uma escala de 5 níveis, codificada de `0x00` a `0x04`
(`src/cli.py:51`):

| Menu | Byte (`0x0A`) | Percentual |
| --- | --- | --- |
| 1 | `0x00` | 0% |
| 2 | `0x01` | 25% |
| 3 | `0x02` | 50% |
| 4 | `0x03` | 75% |
| 5 | `0x04` | 100% |

## 6. Velocidade

A velocidade usa 5 níveis no offset `0x0B`, mas o valor enviado é invertido em
relação ao rótulo da interface:

| Nível | Byte (`0x0B`) | Comportamento |
| --- | --- | --- |
| 1 | `0x04` | Mais lenta |
| 2 | `0x03` | Lenta |
| 3 | `0x02` | Média |
| 4 | `0x01` | Rápida (padrão) |
| 5 | `0x00` | Mais rápida |

O valor `0x01` do relatório base corresponde, portanto, ao nível 4. Antes desta
função, qualquer alteração enviava `0x01` fixo e o teclado voltava para a
velocidade 4. A velocidade só tem efeito visível em modos animados; em
Estático e Desligado ela fica armazenada para o próximo efeito.

## 7. Cor

A cor é informada em HEX no formato `RRGGBB` ou `#RRGGBB`
(`src/colors.py:53`) e dividida em três bytes independentes:

```
FF5733  ->  R = 0xFF, G = 0x57, B = 0x33
```

Cada alteração reenvia o relatório completo, preservando o modo, o brilho e a
velocidade atuais. O mesmo vale para mudanças de modo, brilho e velocidade, que
preservam a cor. O estado é mantido em `KeyboardState` (`src/colors.py:45`),
cujo padrão é Estático (`0x06`), 100% (`0x04`), velocidade 4 (`0x01`) e branco
(`0xFF,0xFF,0xFF`).

## 8. Sequência de envio

```
host                                   teclado
  |  SET_REPORT (Report ID 0x04, 64 B)    |
  |-------------------------------------->|
  |  (sem resposta)                        |
```

Fluxo implementado (`src/cli.py:17`):

1. Localiza o caminho HID pela configuração salva.
2. Abre o dispositivo (`hid.device.open_path`).
3. Monta os 64 bytes a partir do estado atual.
4. Envia com `hid.device.write(report)`.
5. Fecha o dispositivo.
6. Salva o novo estado em `~/.huskynext/config.json`.

O valor retornado por `write()` é o número de bytes efetivamente escritos e é
exibido na CLI.

## 9. Exemplos de relatório

Estado padrão — Estático, 100%, velocidade 4, branco:

```
04 A1 01 06 38 00 00 55 00 06 04 01 00 00 FF FF FF 00 ... 00
```

Cor `FF5733` com modo Estático, 100% e velocidade 4:

```
04 A1 01 06 38 00 00 55 00 06 04 01 00 00 FF 57 33 00 ... 00
```

Arco-íris (`0x0D`) com 50% de brilho e velocidade 5 (mais rápida):

```
04 A1 01 06 38 00 00 55 00 0D 02 00 00 00 FF FF FF 00 ... 00
```

Arco-íris (`0x0D`) com 50% de brilho e velocidade 1 (mais lenta):

```
04 A1 01 06 38 00 00 55 00 0D 02 04 00 00 FF FF FF 00 ... 00
```

Desligado (`0x00`), 0%, velocidade 4, mantendo a cor:

```
04 A1 01 06 38 00 00 55 00 00 00 01 00 00 FF FF FF 00 ... 00
```

## 10. Observações e limitações

- O protocolo é somente de escrita; não há handshake nem leitura de estado.
- O byte de velocidade (`0x0B`) foi identificado por teste no teclado; os
  valores são invertidos em relação ao nível exibido (`0x04` = nível 1,
  `0x00` = nível 5).
- O assistente de teste (`src/setup_wizard.py:37`) envia o estado padrão
  (branco, Estático, 100%), embora a pergunta ao usuário mencione vermelho.
- No Linux, o acesso exige regras `udev` para `usb` e `hidraw` (ver README);
  sem permissão, a abertura do dispositivo falha com `EACCES`/`EPERM`.
- O número de bytes fixos e os offsets podem variar em outros modelos/revisões
  do controlador; a identificação por Usage Page vendor-specific é a principal
  heurística de compatibilidade.
