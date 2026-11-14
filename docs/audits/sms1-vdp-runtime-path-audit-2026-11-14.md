# Auditoria do caminho VDP SMS1 no runtime desktop

**Data:** 14/11/2026
**Commit auditado:** `118b96bf411248173b6215b7bf16aebdf3a66bff`
**Natureza da evidência:** auditoria estática restrita ao snapshot desse commit; não representa inspeção dos commits posteriores nem validação visual independente.
**Escopo:** verificar se o desktop Slint executa o VDP SMS1/315-5124 consolidado, se existe uma rota de renderização legada ativa e quais lacunas podem explicar bugs persistentes de sprites.
**Tipo:** auditoria estática do código + observações preliminares de gameplay reportadas pelo usuário. Não é uma validação visual independente.

## 1. Observações preliminares de gameplay

O usuário reportou na build Release Windows x64:

- velocidade de emulação estável e plena, melhor que versões anteriores;
- fullscreen funcional;
- SRAM funcional em *Phantasy Star*;
- ausência de controles visuais para overscan e Save States;
- os mesmos jogos anteriormente reportados continuam apresentando os mesmos defeitos de sprites, sem alteração perceptível.

Os títulos afetados e a descrição exata de cada artefato visual ainda não foram identificados neste registro. Essas observações são dados de teste manual fornecidos pelo usuário, não resultados reproduzidos pela auditoria.

## 2. Conclusão principal: o VDP SMS1 novo está no caminho ativo

O caminho do host desktop é:

```text
native/sega_desktop/src/main.rs
  create_sms_worker(...)
    SmsTemporalWorker::new_with_cartridge_sram_size(...)
      rust/runtime/sms-runtime/src/worker.rs
        sms_temporal_region_profile(region)
          SmsVdpProfile::new(Sega3155124, ...)
        SmsTemporalMachine::new_with_hardware_profile_and_sram_size(...)
          native/sms_core/src/temporal_machine.rs
            SmsTemporalBus
              native/sms_core/src/temporal_bus.rs
                SmsVdp::new_with_profile(vdp_profile)
                  native/sms_core/src/vdp.rs (re-export)
                    sega_video::sms::SmsVdp
                      native/video_core/src/sms/vdp.rs
```

Evidências de implementação:

1. `native/sega_desktop/src/main.rs` importa `SmsTemporalWorker` de `sega_sms_runtime`; a função `create_sms_worker` cria o worker por `new_with_cartridge_sram_size`.
2. `rust/runtime/sms-runtime/src/worker.rs::sms_temporal_region_profile` seleciona `SmsVdpRevision::Sega3155124` para USA, Brasil, Europa e Japão. A região altera o padrão de vídeo/clock e território, não troca o SMS1 pelo TMS9918A.
3. O worker constrói `SmsTemporalMachine` passando explicitamente esse `SmsVdpProfile`.
4. `native/sms_core/src/temporal_bus.rs` instancia `SmsVdp::new_with_profile(vdp_profile)`.
5. `native/sms_core/src/vdp.rs` é somente uma reexportação do tipo `sega_video::sms::SmsVdp`.
6. O engine efetivo está em `native/video_core/src/sms/vdp.rs`.

Fontes:
- [Desktop host](../../native/sega_desktop/src/main.rs)
- [SMS worker e seleção do perfil por região](../../rust/runtime/sms-runtime/src/worker.rs)
- [SMS temporal bus](../../native/sms_core/src/temporal_bus.rs)
- [Reexport compatível](../../native/sms_core/src/vdp.rs)
- [VDP SMS compartilhado](../../native/video_core/src/sms/vdp.rs)

**Resultado:** não há evidência de que o desktop atual esteja instanciando o antigo TMS9918A do SG-1000 como VDP do Master System. O runtime desktop seleciona explicitamente o ASIC SMS1 315-5124.

## 3. O que ainda existe com nome/path legado

Há duas coisas diferentes que não devem ser confundidas:

### 3.1 VDP TMS9918A do SG-1000

O TMS9918A está em `native/sg1000_core/src/vdp.rs` e é propriedade do runtime SG-1000. O perfil de sistema compartilhado distingue `VideoEngineKind::Tms9918A` de `VideoEngineKind::Mode4`. Esse núcleo não é o VDP instanciado pelo worker SMS desktop.

### 3.2 Renderer de modos TMS/compatibilidade dentro do SmsVdp

O próprio `native/video_core/src/sms/vdp.rs` conserva `render_legacy_scanline_span` e `LegacySpriteLineBuffer`. A seleção ocorre em `render_scanline_for_revision_with_evaluation`:

- se `R0 & 0x04 != 0`, o renderer segue para o caminho SMS Mode 4;
- se `R0 & 0x04 == 0`, chama `render_legacy_scanline_span`, que implementa modos gráficos compatíveis com TMS e a respectiva avaliação antiga de sprites.

Essa é uma rota de compatibilidade dentro do engine SMS compartilhado, não a troca do dispositivo por `Tms9918A`. Portanto, para cada jogo afetado, o valor de R0 precisa ser observado durante o gameplay. Um jogo que mantenha M4 desligado realmente percorre a rota legada de modos gráficos; um jogo com M4 ligado percorre o renderer Mode 4.

### 3.3 Protótipo SMS1 anterior

O protótipo `native/sms_core/src/vdp_sms1.rs` foi arquivado como `legacy/rust/sms_vdp1_prototype.rs`. Ele é referência histórica e não é o tipo construído pelo `SmsTemporalBus`. A consolidação migrou máscaras R5/R6, zoom, colisão e prioridade para o engine compartilhado.

Fonte: [consolidação do VDP SMS1 e Scheduler](../../docs/architecture/scheduler-sms1-vdp-d3d12-consolidation.md).

## 4. Caminho Mode 4 e line buffer atual

No perfil SMS1/315-5124, `SmsVdp::advance()` prepara o buffer inicial de sprites e `complete_scanline()` prepara a seleção da próxima linha. O engine mantém dois resultados separados:

- `Mode4SpriteLineBuffer`: seleção de sprites Mode 4;
- `LegacySpriteLineBuffer`: seleção usada somente se o registrador R0 deixar Mode 4 desabilitado.

A seleção Mode 4 percorre a SAT em ordem, aplica o limite de oito entradas no modo `HardwareLimit`, calcula o índice de overflow e captura metadados Y/linha, X e tile para a linha de destino. O renderer consome essa seleção na composição da linha.

Na baseline auditada, o caminho ativo não é, portanto, o renderer SMS antigo simplesmente reutilizado sem alterações: existe uma seleção Mode 4 persistente entre scanlines, além do perfil 315-5124 e das regras de endereço específicas de R5/R6.

## 5. Limitações confirmadas que ainda podem produzir defeitos de sprites

O novo engine está ativo, mas o pipeline de sprites ainda é parcial:

1. **Boundary de avaliação aproximado:** a SAT é selecionada no fechamento da scanline anterior. A documentação assume explicitamente um modelo scanline-level; não há timestamp sublinha medido para o ASIC.
2. **Fetch incompleto:** X/tile e Y são capturados no buffer; os bytes de pattern continuam sendo consultados durante a composição. Não há fetch individual de X/tile/pattern por dot nem slots de VRAM temporizados.
3. **Sem arbitragem de fetch:** o código não modela a sequência completa de fetch de sprites, contenção de VRAM ou WAIT real associado ao fetch.
4. **Colisão ainda depende da composição:** pixels opacos sobrepostos são detectados durante o renderer; o momento exato de latch/visibilidade do ASIC continua limitado ao modelo de scanline/boundaries observáveis.
5. **Escritas raster:** alterações de SAT/pattern/VRAM durante a linha não têm um pipeline físico de fetch completo. Uma parte da composição ainda consulta a memória no instante do render, não em timestamps independentes de fetch.
6. **Cobertura de ROM:** os testes unitários cobrem regras isoladas, mas não demonstram que os jogos anteriormente afetados tenham sido corrigidos. A campanha genérica de smoke test verifica execução e publicação de frames não pretos, não correção visual dos sprites.

Referências internas:
- [Fase 2 — buffer de sprites](sms1-sprite-line-buffer-phase2-2026-11-14.md)
- [Fase 3 — overflow na avaliação SAT](sms1-sprite-overflow-evaluation-phase3-2026-11-14.md)
- [Fase 5 — colisão e HBlank](sms1-sprite-collision-hblank-phase5-2026-11-14.md)
- [Matriz de evidências temporais](sms1-vdp-temporal-evidence-matrix-2026-11-14.md)
- [Campanha de ROMs in-game](../testing/sms-in-game-rom-smoke-campaign.md)

Não declarar o VDP SMS1 dot-accurate ou integralmente fechado. O caminho ativo é o engine novo, mas a implementação de fetch e a precisão temporal de sprites continuam incompletas.

## 6. Overscan e Save States: APIs existem, integração de GUI não

### Overscan

- `SmsOverscanMode::Off` e `CropLeft8` existem.
- `SmsTemporalWorker::set_overscan_mode` encaminha o modo ao worker.
- O padrão é `Off`.
- O crop é transformação de apresentação 256→248 pixels, não geração de pixels de overscan físico.
- `native/sega_desktop/src/main.rs` não expõe controle nem persiste essa preferência.

### Save States

- `SmsTemporalWorker::save_state` e `load_state` existem como APIs de snapshot em memória.
- A restauração exige estado pausado e valida versões/invariantes.
- A codificação de arquivo, slots persistentes e identidade/checksum portátil da ROM ainda não estão implementados.
- O host Slint não possui ações/menu para salvar ou carregar Save States.
- SRAM `.srm` é armazenamento separado e não substitui Save State.

Fontes:
- [Worker save/load state e overscan](../../rust/runtime/sms-runtime/src/worker.rs)
- [Escopo de software SMS1](sms1-vdp-software-scope-2026-11-14.md)
- [Desktop README](../../native/sega_desktop/README.md)

## 7. Próxima investigação — sem alterar o hardware prematuramente

Antes de corrigir sprites, coletar uma reprodução determinística por jogo:

1. Registrar título do jogo, região selecionada e revisão de ROM/hash local (não enviar ROM comercial).
2. Registrar o sintoma exato: sprite ausente, deslocado, cortado, duplicado, flicker, prioridade incorreta, tile/padrão corrompido ou overflow.
3. Capturar o valor de R0 e confirmar `R0 bit 2` durante a cena afetada.
4. Capturar uma imagem/frame do nosso emulador e a mesma cena no Ares ou outro oráculo independente, com região, frame e sequência de entrada equivalentes.
5. Instrumentar, sem alterar o caminho quente desnecessariamente, as escritas de porta VDP com timestamp do Scheduler, R0/R1/R5/R6, endereço SAT e valores Y/X/tile/pattern relevantes.
6. Criar teste sintético mínimo que reproduza o caso: SAT alterada durante a linha, X/tile, pattern, 8/9 sprites, 8×16, zoom, sprite shift, prioridade e colisão.
7. Corrigir a menor fronteira comprovada e preservar chunking invariance, snapshot/restore e causalidade do Scheduler.

A UI atual não mostra os registradores VDP no snapshot DEBUG do desktop; esse snapshot exibe principalmente registradores Z80 e contadores de frame/tempo. Uma próxima melhoria de diagnóstico deve expor perfil ASIC, R0/M4 e resumo do line buffer no snapshot imutável, sem conceder controle de ciclos à GUI.

## 8. Classificação

| Item | Resultado |
|---|---|
| Desktop usa `SmsTemporalWorker` | CONFIRMADO |
| Worker seleciona SMS1 315-5124 para as regiões expostas | CONFIRMADO |
| `SmsTemporalBus` instancia o `SmsVdp` compartilhado com perfil explícito | CONFIRMADO |
| TMS9918A SG-1000 substitui o VDP SMS no desktop | NÃO |
| Protótipo `vdp_sms1.rs` antigo é o tipo ativo | NÃO; arquivado |
| Rota de renderer TMS/compatibilidade existe dentro do `SmsVdp` | SIM; condicionada a R0 bit 2 desligado |
| Mode 4 possui seleção persistente de sprites entre scanlines | SIM; modelo scanline-level |
| Fetch completo de sprites por dot / arbitragem de VRAM | NÃO |
| Causa dos bugs nos jogos do usuário identificada | NÃO; falta reprodução específica |
| Overscan visível na GUI | NÃO |
| Save State persistente/visível na GUI | NÃO |
| SRAM em *Phantasy Star* | PASS reportado pelo usuário |
| Velocidade estável e fullscreen | PASS reportado pelo usuário |

**Conclusão:** no commit auditado, o VDP SMS1 novo está efetivamente no caminho de produção do desktop. A hipótese de que o host esteja usando o TMS9918A do SG-1000 ou o protótipo SMS1 arquivado é descartada pela cadeia de construção examinada. As limitações do pipeline de fetch/visibilidade temporal e a possível seleção de modos TMS com M4 desligado são hipóteses tecnicamente plausíveis para investigar, mas esta auditoria não demonstra que sejam a causa dos defeitos visuais reportados. A causa específica só poderá ser estabelecida com os títulos, R0 e uma reprodução visual determinística comparada a um oráculo independente.
