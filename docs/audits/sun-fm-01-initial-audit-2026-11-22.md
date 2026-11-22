# SUN-FM-01 — Auditoria inicial do YM2413

**Data:** 2026-11-22  
**Repositório:** https://github.com/leonardo201800478/SUN  
**HEAD da main auditado:** 7ba2d59dc9efdb24deb15171ec560df05cbfab6a  
**Estado:** Sprint aberta; nenhuma aprovação de gameplay foi emitida.

## 1. Resumo executivo

A auditoria confirma que o YM2413 está integrado ao caminho temporal do SMS e que a síntese serial é executada dentro do avanço do barramento. O Scheduler/runtime continua sendo a autoridade temporal. Não há evidência que justifique mover o clock do chip para uma thread independente.

As otimizações já publicadas (#1194 e #1199) melhoraram caminhos específicos, mas não existe captura A/B de gameplay que confirme 60 FPS estáveis. Os valores relatados anteriormente (OutRun ~52 FPS; Kenseiden ~50–51 FPS) continuam sendo relatos, não baseline medido.

Foram identificadas três causas plausíveis para sintomas distintos:
- **Ausência de áudio FM:** o desktop inicia em FM Auto e a lista SHA-256 do catálogo Auto está vazia; Auto resolve para saída FM mutada. A política de mute não interrompe o clock nem o processamento interno do chip.
- **Ausência intermitente de FM:** qualquer amostra FM desconhecida (None) faz o worker descartar o lote FM inteiro, limpar a fila FM e reiniciar o resampler. A regra é intencionalmente conservadora, mas sua frequência em jogos reais ainda não foi medida.
- **Volume inadequado:** o caminho PCM usa uma reconstrução digital MO/RO cuja correspondência com a saída analógica real ainda não está calibrada. O manual Yamaha descreve MO/RO como saídas pulsadas que exigem circuito externo de integração/amplificação.

## 2. Estado Git e validação

### Main e entregas já integradas

| PR | Entrega | Estado |
|---|---|---|
| #1194 | Caminho in-place no runtime SMS; remoção de deep clones | Integrada |
| #1199 | Remoção de dispatch serial redundante | Integrada |
| #1200 | Roteiro/documentação A/B Kenseiden | Integrada |
| #1201 | Correção da captura de saída PowerShell no benchmark | Integrada |
| #1203 | Correção do caminho do script A/B na documentação | Integrada |

HEAD atual da main: 7ba2d59dc9efdb24deb15171ec560df05cbfab6a.

### PRs ainda abertas

- [#1188](https://github.com/leonardo201800478/SUN/pull/1188): smoke tests com ROM real. Branch originada de uma base antiga; não duplicar nem integrar sem reconciliar com #1193.
- [#1192](https://github.com/leonardo201800478/SUN/pull/1192): adaptador/comparador externo de saída YM2413. CI reportado como sucesso; ainda não integrada.
- [#1193](https://github.com/leonardo201800478/SUN/pull/1193): diagnóstico de escritas FM e smoke test com ROM real. CI reportado como sucesso; a tentativa de merge foi recusada pela regra do repositório porque o check obrigatório Rust quality and unit regression foi considerado expected.
- [#1202](https://github.com/leonardo201800478/SUN/pull/1202): reutilização do buffer PCM final e envio CPAL por referência. SMS Runtime, CPAL Windows, PSG/audio-runtime, higiene documental e workflow validation terminaram com sucesso. A tentativa de merge também foi recusada pelo mesmo check obrigatório Rust quality and unit regression, apesar do job aparecer como sucesso. Foi solicitado rerun do job agregador; não contornar a proteção da branch.
- [#1205](https://github.com/leonardo201800478/SUN/pull/1205): métricas por frame de amostras FM válidas/desconhecidas/não finitas, fallback, peak/RMS e amostras alinhadas. Também evita propagar NaN/Inf ao mixer. CI pendente; não integrada.

O roteiro A/B está em **scripts/benchmark/ym2413-real-game-ab.ps1**; a instrução de execução em **docs/testing/ym2413-real-game-ab-2026-11-22.md** foi corrigida pela PR #1203.

**Não existe artefato de gameplay A/B publicado ou recebido nesta auditoria.** Os testes de ROM real permanecem opt-in/ignorados no CI hospedado e dependem das ROMs locais do usuário.

## 3. Arquitetura observada

### Caminho temporal e áudio atual

CPU / barramentos / MachineScheduler  
→ SmsTemporalBus::advance_ym2413_inner  
→ Ym2413Core::advance_input_clocks_with_pg_service_in_place  
→ Ym2413PgSerialService (PG, operador, EG, LFO, feedback e pipeline digital MO/RO)  
→ SmsTemporalBus (amostras FM Option<f64>)  
→ SmsTemporalWorker::AudioRendererState (alinha FM nominal de 49.716 Hz com PSG a 48 kHz)  
→ NativeAudioSession (mixer, saída analógica opcional, resampler final)  
→ PcmSink → NativeCpalOutput → SPSC FIFO → callback CPAL → dispositivo.

O worker de emulação executa o frame; a saída CPAL consome em thread própria. A síntese FM não possui thread independente. Isso preserva a causalidade entre escritas Z80, barramento e slots.

### Vídeo observado

SMS VDP framebuffer  
→ framebuffer_with_overscan(Off): cópia RGB  
→ VideoFrame / mailbox  
→ host Slint/WGPU  
→ RGB888 para RGBA8: novo buffer  
→ WGPU Queue::write_texture.

O custo do vídeo deve ser medido separadamente. As cópias por frame são reais, mas não há evidência de que expliquem isoladamente a queda associada ao FM.

## 4. Matriz de achados

| ID | Arquivo / função | Evidência | Classificação e impacto | Correção / prova exigida |
|---|---|---|---|---|
| A01 | native/sega_desktop/src/main.rs — FmMode, resolve_sms_fm_configuration | FM_AUTO_COMPATIBLE_ROM_SHA256 está vazio; Auto retorna Muted quando não há hash cadastrado. | **Confirmado.** Explica ausência de áudio quando o usuário está em Auto; não explica FPS porque o chip continua sendo clockado. | Configurar perfil Japanese SMS integrado + SMS1 + FM On nos testes. Só adicionar hashes ao Auto após trace real comprovar uso FM. |
| A02 | rust/runtime/sms-runtime/src/worker.rs — AudioRendererState::prepare_fm_mix, prepare_psg_fallback | Um único None na fatia FM aciona fallback de todo o lote, limpa FM pendente e reinicia o resampler. Existe teste sintético que confirma essa política. | **Comportamento confirmado; frequência real desconhecida.** Pode causar silêncio/dropouts FM se ocorrer em jogo. | PR #1205 adiciona contagens e métricas condicionadas ao consentimento diagnóstico. Após CI/merge, coletar em Kenseiden e Phantasy Star antes de alterar a política. |
| A03 | native/sms_core/src/temporal_bus.rs — advance_ym2413_inner; native/ym2413/src/lib.rs — advance_input_clocks_with_pg_service_in_place | O runtime usa caminho in-place e mantém o avanço do chip no fluxo temporal do barramento. Uma rotação contém 18 slots; o contrato do core usa 4 clocks de entrada por slot. | **Arquitetura confirmada.** O custo serial é candidato principal de profiling, não defeito provado. | Medir ym2413_advance_elapsed_ns p50/p95/p99 em gameplay. Não mover clock para thread independente. |
| A04 | native/ym2413/src/lib.rs — clock_one_slot_with_service; native/ym2413/src/pg_serial.rs — before_eg_update, after_eg_update, latch_next_parameters | O contexto por slot carrega imagem de registradores e estado EG; há resolução de slot nas duas fases. O estado pós-EG é usado por teste para provar a ordem do sequenciador. | **Candidato de custo, não redundância confirmada.** Remover cópias/dispatch sem preservar o contrato pode quebrar timing. | Perfil por função e benchmark diferencial. Não remover post_state sem substituir a prova temporal existente. |
| A05 | native/audio_runtime/src/session.rs — process_sources_with_policy; native/audio_runtime/src/lib.rs — PcmResampler::process | A sessão chamava o wrapper que criava um novo Vec<f64> por lote, embora process_into exista. | **Ineficiência confirmada.** Corrigida em candidata #1202; não medir ganho de FPS até gameplay. | #1202 adiciona buffer persistente, process_into e teste de capacidade reutilizada. |
| A06 | native/audio_output/src/lib.rs — PcmSink for NativeCpalOutput; native/audio_output/src/pcm_fifo.rs; native/sega_app/src/app.rs — configure_audio_output | CPAL converte mono para StereoFrame em scratch reutilizável e publica em SPSC; callback consome FIFO e preenche silêncio em underrun. Capacidade configurada: 512 × 8 = 4.096 frames. | **Arquitetura confirmada.** A capacidade equivale a ~85,3 ms em 48 kHz; isso é capacidade máxima, não latência medida. | Medir ocupação real, high-water, drops e underruns. Só reduzir capacidade após medir tamanho dos blocos e estabilidade. |
| A07 | native/video_core/src/sms/vdp.rs — framebuffer_with_overscan; native/sega_desktop/src/main.rs — convert_rgb888_to_rgba8 | Há cópia RGB do framebuffer e alocação RGBA por frame. | **Ineficiência confirmada; impacto desconhecido.** Pode consumir margem do orçamento de 16,67 ms. | Medir video_publish_elapsed_ns, conversão RGB→RGBA e Queue::write_texture; otimizar somente se p95/p99 justificarem. |
| A08 | native/sms_core/src/temporal_bus.rs — capture_ym2413_output_frame; native/ym2413/src/output_pipeline.rs | O caminho PCM usa saída digital de referência MO/RO e média por rotação. O adaptador de domínio medido permanece separado. | **Limitação de fidelidade confirmada.** Amplitude/volume ainda não calibrados contra saída analógica. | Captura real de MO/RO e validação do filtro/ganho; não alterar normalização por palpite. |

## 5. Contratos de hardware e decisões

- O YM2413 tem um único operador físico reutilizado 18 vezes para nove canais. O manual especifica fsam = fM/72, aproximadamente 50 kHz para clock de 3,579545 MHz.
- MO/RO são saídas pulsadas/source-follower; o manual descreve a necessidade de circuito externo de integração/amplificação. A média PCM atual é uma aproximação digital, não validação de circuito analógico.
- O Scheduler continua autoridade do tempo. Não permitir thread FM avançar clocks independentemente.
- Não criar thread FM agora. A dependência por slot e a taxa de eventos podem tornar a fila mais cara que a execução in-place. Reavaliar somente se o perfil demonstrar benefício líquido.
- Não aplicar SIMD aos operadores, feedback, PG ou EG sequenciais. Avaliar AVX2 no mixer/resampler e conversão de pixels apenas se profiling apontar custo relevante. Preservar caminho escalar e testes diferenciais; FMA altera arredondamento.
- A latência da FIFO não pode ser inferida da capacidade. Para provar menos de 10 ms, registrar ocupação real em frames e converter pela taxa efetiva do dispositivo.

## 6. Instrumentação e estratégia de benchmark

O diagnóstico já emite:
- ym2413_advance_elapsed_ns
- step_frame_elapsed_ns
- audio_prepare_elapsed_ns
- audio_submit_elapsed_ns
- video_publish_elapsed_ns
- frame_processing_elapsed_ns
- pacing_budget_ns e métricas de espera

A coleta deve produzir p50/p95/p99, média, desvio e máximo por execução. Os logs precisam incluir métricas CPAL e host WGPU.

### Campanhas obrigatórias

1. **Kenseiden Japan** — benchmark principal. SHA-256 conhecido: 7a9c056edfb1b973482430a41074b0b0a076ee4591d5df8f19007e7058cadc93.
2. **OutRun** — confirmar os relatos de 50–52 FPS em gameplay e adicionar à matriz A/B.
3. **Phantasy Star Japan Rev 1** — confirmar escritas F0/F1/F2, saída FM não nula e continuidade. A captura anterior não produziu amostras FM não nulas; não assumir ausência de defeito sem o novo trace.

O A/B atual compara commits pré/pós-#1199 em ABBA, com 30 s de aquecimento e 180 s de captura. Ele ainda não foi executado com ROM local. O resultado de microbenchmark de ~56,5 µs/frame não substitui gameplay.

### Testes existentes

- YM2413 core: testes de slots, EG, PG, snapshots, saída digital e benchmark in-place.
- SMS runtime: alinhamento FM/PSG e fallback para rotação desconhecida.
- Audio runtime: mixer, resampler, invariância por blocos e saída PCM.
- CPAL: FIFO SPSC, conversão de formato, underrun/overflow e testes Windows.
- Smoke test com ROM real: PR #1193, opt-in/ignorado no CI; depende de ROM local.
- A/B Kenseiden: script PowerShell, execução física ainda pendente.

## 7. Dependências verificadas

Versões do Cargo.lock na main auditada:
- Slint 1.18.1.
- WGPU 30.0.1.
- CPAL 0.18.2.
- rtrb 0.3.6.

Na data da auditoria:
- Slint 1.19.0 já é a versão estável mais recente, mas o projeto mantém a decisão arquitetural explícita de permanecer em 1.18.x; o lock está em 1.18.1.
- WGPU 30.0.2 foi publicado em 12/11/2026 como patch da série 30.x; o lock está em 30.0.1. A atualização deve ser separada da sprint de áudio para não contaminar a comparação A/B.
- CPAL 0.18.2 é o release estável mais recente consultado e corresponde à versão arquitetural fixada. O release adiciona ErrorKind::Xrun para AAudio, CoreAudio e PipeWire; em WASAPI, a nota cita captura, não saída. Portanto, no Windows, manter métricas internas da FIFO/underrun em vez de depender exclusivamente de notificações de Xrun do CPAL.

Referências:
- https://www.smspower.org/Development/YM2413
- https://www.smspower.org/maxim/Documents/YM2413ApplicationManual
- https://github.com/RustAudio/cpal/releases
- https://github.com/gfx-rs/wgpu/releases
- https://github.com/slint-ui/slint/releases

## 8. Plano ordenado

1. Resolver o reconhecimento do check obrigatório do GitHub sem contornar branch protection; integrar #1192/#1193 e #1202 após checks válidos.
2. Integrar #1205 após CI e executar Kenseiden A/B no Windows local com ROM verificada e FM On.
3. Executar Phantasy Star e determinar se o problema é ausência de escritas, saída desconhecida, política de mute ou síntese.
4. Executar OutRun para confirmar desempenho sob gameplay real.
5. Usar p95/p99 para localizar o gargalo: core YM2413, preparo de áudio, CPAL, vídeo ou host.
6. Só então avaliar cache de dispatch/configuração, filas/blocos menores, AVX2/FMA e eventual processamento paralelo de PCM.
7. Atualizar scorecard com evidências de gameplay, fidelidade e latência.

**Critério de conclusão permanece inalterado:** 60 FPS estáveis, frame time dentro de 16,67 ms, áudio contínuo, sem underruns recorrentes, amplitude adequada, latência interna inferior a 10 ms, fidelidade diferencial e validação real dos três jogos. Até lá, SUN-FM-01 permanece aberta.
