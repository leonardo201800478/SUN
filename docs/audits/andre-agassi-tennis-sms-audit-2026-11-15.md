# Andre Agassi Tennis (SMS) — auditoria da fronteira HINT/VINT

**Data da auditoria:** 2026-11-15
**Sistema:** Master System 1 / Sega 315-5124
**ROM analisada:** Andre Agassi Tennis (Europe, Brazil) (En), 262.144 bytes
**Escopo da correção:** contagem HINT/R10, pedido VINT, flag VBlank e publicação do frame
**Estado deste PR:** correção implementada em branch; validação automatizada deste commit depende do CI.

## 1. Resumo executivo

A falha observada no Andre Agassi Tennis é compatível com uma divergência na fronteira vertical do VDP SMS1. O core anterior:

- só decrementava/avaliava HINT enquanto `scanline < visible_height`, excluindo a primeira linha não visível;
- solicitava VINT e marcava o bit VBlank do status na linha `visible_height`.

Para o modo de 192 linhas visíveis, isso excluía a linha 192 da contagem HINT e antecipava a classificação de VBlank. A ROM programa `R10=$C0` (192), de modo que a diferença de uma linha é diretamente relevante.

A correção proposta neste branch:

1. inclui a linha 192 na janela HINT SMS1 (`scanline <= visible_height`);
2. mantém a publicação do framebuffer concluído na entrada da linha 192, em T16;
3. desloca o pedido VINT para T13 da linha 193;
4. desloca o bit VBlank do status para T16 da linha 193.

Isso permite que o ISR que atende o HINT de R10=$C0 leia o status antes de VINT/VBlank e classifique corretamente a interrupção. A alteração é restrita ao caminho SMS1; não modifica Z80, mapper, Scheduler de produção, PSG, GUI ou ROM.

## 2. ROM e evidência fornecida

Identificação informada no pacote:

| Campo | Valor |
|---|---|
| Tamanho | `262144` bytes |
| SHA-1 | `81dbacad4739b98281c750d9af21606275398fc8` |
| SHA-256 | `522a2c8b34236f40a1f71afda104c1a27f11c52fdd1ffd7f63b8ef4121fe4224` |
| CRC32 | `F499034D` |

A ROM comercial permanece fora do repositório.

A captura visual incluída no pacote mostra uma cena de créditos/personagem, não a tela de título. Ela é evidência de que a execução avançou para além do estado inicial preto, mas não prova que o logotipo/tela de título foi reproduzido corretamente nem que o jogo inteiro está funcional.

## 3. O teste de boot da Manus e o estímulo de entrada

O teste da Manus é um teste de boot com estímulo determinístico de entrada. O código da sonda incluído no pacote chama `set_controller(1, ...)` e mantém `button_1` ativo quando `frame % 12 == 6`.

Esse pulso periódico é intencional: simula uma entrada do jogador para que a ROM não permaneça indefinidamente em uma tela que aguarda confirmação. Não é uma falha metodológica nem invalida a evidência de progressão. O teste deve ser descrito como **boot com entrada sintética periódica**, e não como boot sem controles.

Há, porém, duas inconsistências adicionais na sonda que precisam ser tratadas antes de usar seus números como evidência final:

- A sonda também escreve artificialmente `$AA` em RAM em `$C920` antes da execução. Como esse endereço pertence à RAM da máquina e o pacote não demonstra que a ROM não o consulta, esse seed não deve ser usado em uma execução de aceitação. Remover a escrita e preservar a RAM inicial fornecida pelo runtime.
- O campo chamado `framebuffer_nonzero` conta bytes/canais RGB não nulos, não pixels. A expressão correta para contar pixels não pretos é:

```rust
let non_black_pixels = framebuffer
    .chunks_exact(3)
    .filter(|pixel| pixel.iter().any(|channel| *channel != 0))
    .count();
```

Na captura PNG de 256×192 incluída no pacote, a contagem é **7.886 pixels não pretos** e **16.336 canais RGB não nulos**. Portanto, o valor 16.336 não deve ser descrito como quantidade de pixels.

O relatório original dizia que a sonda não injetava controles, mas o código mostra o pulso periódico de `button_1`. Esta documentação corrige essa descrição. A escrita artificial em `$C920` também é explicitada como limitação.

## 4. Diagnóstico técnico da fronteira

A primeira divergência identificada é a inclusão/exclusão da linha não visível de índice `visible_height`:

| Evento SMS1 | Comportamento anterior | Comportamento deste branch |
|---|---|---|
| HINT/R10 | `scanline < height` | `scanline <= height` |
| R10=$C0 | não expira na linha 192 | expira na linha 192, T14 |
| Pedido VINT | linha 192, T13 | linha 193, T13 |
| Publicação do frame | junto com entrada antiga em VBlank | linha 192, T16 |
| Bit VBlank no status | linha 192, T16 | linha 193, T16 |

O ISR do teste SMS VDP Test diferencia HINT e VINT por leitura do status. Se o bit VBlank já estiver ativo quando o handler atende o HINT de R10=$C0, a interrupção pode ser classificada como VINT. A correção separa os eventos na linha/T-state, permitindo que o HINT de fronteira seja observado antes do pedido VINT e do flag VBlank.

A hipótese é corroborada pelas implementações comparadas no pacote (GearSystem 3.9.23 e Ares), e pelo teste `RunTest33` (“Line IRQ VCount”) do SMS VDP Test v1.31, que inclui explicitamente R10=$C0. Ares/GearSystem são referências comparativas, não oráculos absolutos do hardware.

## 5. Alterações de código neste branch

- `native/video_core/src/sms/vdp.rs`
  - HINT inclui a primeira linha não visível.
  - VINT passa para T13 da linha seguinte.
  - publicação do framebuffer é separada do flag VBlank;
  - regressões unitárias cobrem R10=192, HINT antes de VINT, publicação do frame e flag VBlank posterior.
- `native/sms_core/src/temporal_bus.rs`
  - boundary de VINT atualizado para a linha 193;
  - novo teste exercita R10=192 através do Scheduler e restaura snapshot um T-state antes da expiração.
- `native/video_core/tests/sms1_vdp_contract.rs`
  - contrato público da leitura de status exige VBlank na linha 193.

A mudança não altera o caminho SMS2. O helper `enter_vblank` permanece disponível para o caminho legado que já o utiliza.

## 6. Evidências reportadas pela Manus — não reexecutadas independentemente neste PR

O relatório incluído no pacote informa:

- `cargo fmt --all -- --check`: PASS;
- `cargo test --release -p sega-video`: PASS, 100 testes unitários + 15 contratos;
- `cargo test --release -p sega-sms-core`: PASS, 496 testes em 55 suites;
- SMS VDP Test v1.31 / `RunTest33`: PASS em PAL e PAL-M, com R10=$C0 observado no handler;
- Andre Agassi Tennis após 1.200 frames: `C122=$0C`, `R1=$C0`, com imagem não preta em PAL e PAL-M.

Esses resultados são **relatados pela Manus**. O pacote não contém a saída bruta completa dos comandos de teste nem o log bruto de 1.200 frames após a correção. O log de trace de 120 frames presente no pacote registra o estado anterior à correção (C122=$03 e framebuffer preto). Assim, os resultados reportados não são promovidos aqui a validação independente.

O código da sonda incluído injeta o pulso de botão 1 e semeia `$C920`; por isso, a execução de aceitação deve ser repetida após remover o seed de RAM, mantendo explicitamente o estímulo de controle documentado. Também deve corrigir a métrica de pixels.

## 7. Matriz de validação

| Verificação | Estado |
|---|---|
| Regressão unitária HINT R10=192 antes de VINT/status | adicionada neste branch; aguardando CI |
| VINT T13 / status T16 na linha 193 | adicionada neste branch; aguardando CI |
| Chunking e snapshot/restore no boundary de VINT | atualizada neste branch; aguardando CI |
| R10=192 através do Scheduler + snapshot/restore | adicionada neste branch; aguardando CI |
| Contrato público de status VBlank | atualizado neste branch; aguardando CI |
| SMS VDP Test v1.31 RunTest33 PAL/PAL-M | PASS reportado pela Manus; sem log bruto no pacote |
| Andre Agassi 1.200 frames PAL/PAL-M | progresso reportado; sem log bruto de 1.200 frames no pacote |
| Reexecução da sonda sem seed artificial em $C920 | pendente |
| Reexecução com métrica RGB por pixel corrigida | pendente |
| Tela de título exata no runtime WGPU | pendente |
| Segundo jogo-controle independente | pendente |
| Captura/avaliação de áudio via CPAL e referência | pendente |
| NTSC após a correção completa | pendente |
| Hardware físico | não executado |

## 8. Critérios para aceitar a correção

1. CI verde no SHA do branch, incluindo `sega-video` e `sega-sms-core`.
2. Executar SMS VDP Test v1.31 / RunTest33 em PAL, PAL-M e NTSC, registrando a leitura do handler e o resultado de R10=$C0.
3. Reexecutar Andre Agassi por 1.200 frames em PAL/PAL-M/NTSC com pulso de botão 1 documentado, sem escrever em RAM do jogo, com trace de PC/C122/registradores VDP/IRQ e contagem RGB por pixel.
4. Capturar a tela resultante e classificar corretamente a cena alcançada (título, menu, créditos/personagem), sem inferir uma tela a partir de outra.
5. Executar um segundo jogo SMS independente como controle de regressão.
6. Avaliar áudio com captura PCM e referência comparável antes de atribuir o chiado ao PSG.
7. Confirmar que SMS2 e SG-1000 não sofreram regressão.

## 9. Referências

- SMS VDP Test v1.31: https://www.smspower.org/Homebrew/SMSVDPTest-SMS
- Código de `RunTest33` / Line IRQ VCount: https://github.com/FluBBaOfWard/SmsVdpTest/blob/490c00b38c83012dee5a7a582097f080c210a3e7/testmain.asm#L3519-L3596
- Ares, VDP IRQ e vlines: https://github.com/ares-emulator/ares/blob/0b928930af4cc59ed99e942996f0d3c815379388/ares/ms/vdp/vdp.cpp#L80-L90 e https://github.com/ares-emulator/ares/blob/0b928930af4cc59ed99e942996f0d3c815379388/ares/ms/vdp/vdp.cpp#L185-L197
- GearSystem 3.9.23, HINT/VINT e VBlank: https://github.com/drhelius/Gearsystem/blob/3.9.23/src/Video.cpp#L425-L478 e https://github.com/drhelius/Gearsystem/blob/3.9.23/src/Video.cpp#L503-L518
- Issue de investigação anterior: https://github.com/leonardo201800478/sega-emulator/issues/855
