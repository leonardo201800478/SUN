# Retirada dos hosts anteriores — Slint / WGPU / CPAL

**Revisão:** 2026-11-22 (pós-merge #1251 e limpeza de hosts)
**Arquitetura normativa:** ADR 0011 — Slint, WGPU e CPAL
**Escopo atual:** desenvolvimento exclusivo do Sega Master System.

## 1. Plataforma mantida

| Camada | Tecnologia |
|---|---|
| GUI desktop | Slint 1.18.x |
| API de renderização | WGPU 30.x |
| Backend gráfico inicial | Vulkan no Windows x64 |
| Saída física de áudio | CPAL 0.18.2 |
| Entrada/gamepads | SDL3 |
| Hardware, CPU, VDP, PSG, barramentos e tempo | Rust; MachineScheduler como autoridade temporal |

Não manter aplicações desktop paralelas, APIs gráficas alternativas ou adaptadores sem consumidor real.

## 2. Estado da retirada

A PR #1251 foi integrada à main (commit `16097f0ea2df6c97bc553ff88a3b9b770d61519f`) e removeu do caminho do produto:

- a aplicação desktop antiga que estava fora do workspace ativo;
- a implementação e as sessões do backend gráfico anterior;
- os métodos do `SmsTemporalWorker` que controlavam diretamente esse backend;
- a fila FIFO de frames que existia exclusivamente para a sessão removida;
- as features gráficas específicas da dependência Windows;
- exemplos de smoke test e documentação operacional exclusiva do backend aposentado.

ADRs, propostas e auditorias datadas permanecem preservadas como histórico, com classificação explícita de `SUPERSEDED`/`HISTÓRICO`. Esses registros não reativam o backend e não constituem validação da arquitetura Slint/WGPU.

A integração preserva `VideoFrame`, `FrameMailbox` e a publicação de frames do runtime SMS. A limpeza subsequente removeu também o host independente `native/sms_debugger`, os manifests/fontes egui e as dependências exclusivas `eframe`/`rfd`. O núcleo `sega-debugger-core` continua ativo por ser consumido pelo SMS runtime. O host Slint/WGPU continua consumindo o contrato de frames sem transferir objetos gráficos para o runtime.

**Estado:** integrada na `main`. A PR #1252 reúne a remoção do host de debugger egui, documentação operacional obsoleta e scripts de build que apontavam para o host eframe inexistente.

## 3. Preservação obrigatória

A retirada não altera:

- Z80 e sua semântica;
- MachineScheduler, clocks ou ordenação temporal;
- SMS VDP, PSG, YM2413, barramentos ou mappers;
- SDL3/input runtime;
- CPAL e o pipeline de áudio;
- determinismo, save states ou replay.

A GUI continua limitada à apresentação e ao controle coarse-grained da sessão. GPU, VSync e pacing de tela não podem avançar ou corrigir o tempo emulado.

## 4. Validação necessária

Na PR #1251, os gates automatizados no SHA final concluíram com sucesso. A validação física do host Slint/WGPU permanece um gate separado:

1. verificar compilação do workspace e do alvo Windows x64;
2. executar testes focados de `sega-video-runtime` e `sega-sms-runtime`;
3. executar `cargo fmt --check` e Clippy aplicável;
4. confirmar que o workflow de lockfiles não referencia mais a aplicação removida;
5. validar a integração Slint/WGPU no Windows com ROM real;
6. confirmar apresentação contínua, resize, encerramento e ausência de regressão no frame pacing;
7. registrar os resultados no SHA exato da PR.

CI não substitui validação física de vídeo, áudio ou controles.

## 5. Invariantes

- MachineScheduler continua sendo a única autoridade do tempo emulado.
- Renderização, áudio físico e input não alteram ciclos do hardware.
- O runtime SMS permanece executável sem device gráfico.
- O contrato de frames é simples e independente da API gráfica.
- Nenhuma camada de compatibilidade ou abstração de backend será adicionada sem consumidor e necessidade demonstrados.
- Não reabrir componentes validados sem defeito ou regressão reproduzível.

## 6. Próxima etapa

Concluir a limpeza de dependências externas e internas com base no grafo Cargo e no uso real dos crates. Não remover `sega-debugger-core`, runtime SMS, Scheduler ou componentes de hardware compartilhados. A validação física Vulkan/CPAL/SDL3 continua separada dos gates de compilação.
