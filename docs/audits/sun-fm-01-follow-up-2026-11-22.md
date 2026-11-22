# SUN-FM-01 — Execução das recomendações da auditoria

**Data:** 2026-11-22
**Base de trabalho:** main pós-merge #1206, `209726445c9a64f06281e01d0aa10fb85bca2831`
**Estado:** acompanhamento em andamento; sem aprovação de gameplay ou conclusão da sprint.

## 1. Ações executadas

### 1.1 Reuso do buffer PCM final — integrado

A PR #1206 foi revisada e integrada por squash.

- `NativeAudioSession` mantém `resampled_samples` reutilizável e chama `PcmResampler::process_into` nos caminhos digital e analógico.
- `PcmSink::write_samples_ref` permite entrega emprestada; o default mantém compatibilidade com sinks antigos.
- `NativeCpalOutput` sobrescreve o método por referência e reutiliza `frame_scratch` para conversão mono→estéreo.
- A mudança não toca no Scheduler, clocks, mixer, filtros, política de FIFO ou sequência de eventos.
- Checks do HEAD #1206: SMS Runtime Release format/Clippy/tests, Windows x64 CPAL, PSG/audio-runtime e regressão de workspace reportados como sucesso.

**Limite da evidência:** isso remove a alocação do vetor final por lote no caminho CPAL após aquecimento/capacidade suficiente. O método default de `PcmSink::write_samples_ref` ainda copia para manter compatibilidade de sinks que só implementam `write_samples(Vec<f64>)`. Não foi medido ganho de FPS nem contagem global de alocações em gameplay.

### 1.2 Diagnóstico de amostras FM — em validação

A implementação foi reaplicada sobre a main pós-#1206 na PR #1208, evitando a base obsoleta da primeira versão #1207.

- Contadores por frame: amostras de origem, finitas, desconhecidas (`None`), não finitas (NaN/Inf), não nulas e alinhadas ao mixer.
- Peak/RMS da fonte FM, calculados somente quando a coleta diagnóstica está autorizada.
- Indicador de fallback PSG quando a fatia FM contém dados inválidos.
- NaN/Inf não são propagados ao mixer; o bloco usa fallback seguro preservando o PSG.
- Testes cobrem amostras válidas, rotação desconhecida e NaN/Inf.

A PR #1208 está aberta e seus workflows estavam em execução no momento deste relatório. #1207 é duplicata baseada antes do merge #1206 e deve ser encerrada após confirmar #1208.

**Limite da evidência:** métricas sintéticas/testes não demonstram a frequência de amostras inválidas em Kenseiden, OutRun ou Phantasy Star. A nova métrica não corrige a causa de amostras desconhecidas; torna o comportamento observável.

### 1.3 Correções documentais — preparadas

Na branch documental desta etapa:
- O runbook A/B remove a sequência literal `\\n` na lista de artefatos. O caminho já estava corrigido para `scripts/benchmark/ym2413-real-game-ab.ps1`.
- O contrato OUT-A passa a descrever o transporte real: `StereoFrame([f64; 2])`; a conversão para o formato negociado ocorre no callback CPAL.
- O scorecard registra o HEAD atual, a integração #1206, a nova instrumentação e a situação real das PRs antigas.

Nenhum teste de Rust é apropriado para essas alterações exclusivamente documentais.

## 2. Coordenação das PRs de ROM real

As PRs #1188 e #1193 alteram os mesmos arquivos:
- `.github/workflows/sms-debugger-runtime.yml`;
- `Cargo.lock`;
- `rust/runtime/sms-runtime/Cargo.toml`;
- `rust/runtime/sms-runtime/tests/fm_in_game_smoke.rs`.

#1188 introduz o harness de gameplay com Kenseiden e Phantasy Star, captura de fontes FM/PSG, mixer/resampler e campanha CPAL. #1193 adiciona ao mesmo harness um probe de até 4.096 instruções para escritas nas portas F0/F1/F2, reseta a máquina após o probe e inclui dependências necessárias.

Ambas estão em bases antigas e não mergeáveis contra a main atual. Não fazer merge independente nem escolher uma versão de `Cargo.lock` antiga. A ação correta é consolidar o conteúdo mais completo em uma nova branch criada da main atual, resolver dependências/lockfile com o workspace atual, executar os checks e abrir uma única PR. O teste continua opt-in e exige ROMs legais locais; nenhuma ROM deve ser commitada.

## 3. Próximas ações ordenadas

1. Confirmar CI completa e revisar manualmente #1208; integrar somente com checks verdes na base atual. Encerrar #1207 como substituída.
2. Publicar a PR documental com as duas correções e o scorecard atualizado.
3. Consolidar #1188 + #1193 em um único harness sobre main, mantendo a identidade SHA-256 das ROMs e o probe de barramento separado da campanha de áudio.
4. Reaplicar/revisar #1190 e #1192 separadamente. A documentação de compatibilidade não deve habilitar Auto sem evidência por ROM; o comparador externo não estabelece sozinho o código físico do DAC.
5. Executar testes Release com ROMs locais: Kenseiden como carga principal, Phantasy Star para diagnóstico de inicialização/saída e OutRun para desempenho.
6. Registrar p50/p95/p99 de YM2413, processamento de frame, áudio e host; comparar underruns/drops e amostras FM desconhecidas.
7. Medir loopback físico CPAL, backlog/high-water e latência ponta a ponta. A capacidade da FIFO não é latência.
8. Só após localizar o custo dominante avaliar mudanças adicionais no core, batching, AVX2/FMA ou paralelismo. Não criar thread FM que avance o clock do chip.

## 4. Estado de aceite

| Critério | Estado |
|---|---|
| Alocação final por lote removida do caminho CPAL comum | Implementado e CI da PR #1206 aprovado |
| Diagnóstico de amostras FM inválidas | Implementado em PR #1208; CI pendente |
| Harness real-game consolidado e integrado | Pendente |
| Kenseiden / OutRun / Phantasy Star em gameplay | Pendente |
| Áudio físico contínuo e sem underruns recorrentes | Pendente |
| Amplitude/volume e clipping calibrados | Pendente |
| Latência interna <10 ms e latência física medida | Pendente |
| 60 FPS sustentados / frame time ≤16,67 ms em jogo real | Pendente |
| Diferenciais de fidelidade e determinismo | Cobertura parcial; execução/aceite pendentes |

**Conclusão:** as primeiras recomendações de baixo risco foram executadas e a alocação por lote no caminho CPAL foi corrigida. A Sprint SUN-FM-01 permanece aberta; nenhuma alegação de 60 FPS, baixa latência ou fidelidade final é feita.
