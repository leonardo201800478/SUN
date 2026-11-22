# SUN-FM-01 — Execução das recomendações da auditoria

**Data:** 2026-11-22
**Base de trabalho:** main pós-merge #1206, #1208, #1213, #1216 e #1217, `53bedb2fd581e2605197a2d11b5cf77fea94e8e4`
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

### 1.2 Diagnóstico de amostras FM — integrado

A implementação foi reaplicada sobre a main pós-#1206 na PR #1208, evitando a base obsoleta da primeira versão #1207.

- Contadores por frame: amostras de origem, finitas, desconhecidas (`None`), não finitas (NaN/Inf), não nulas e alinhadas ao mixer.
- Peak/RMS da fonte FM, calculados somente quando a coleta diagnóstica está autorizada.
- Indicador de fallback PSG quando a fatia FM contém dados inválidos.
- NaN/Inf não são propagados ao mixer; o bloco usa fallback seguro preservando o PSG.
- Testes cobrem amostras válidas, rotação desconhecida e NaN/Inf.

A PR #1208 passou nos checks SMS Runtime Release format/Clippy/tests e Rust workspace regression e foi integrada. #1207 foi encerrada como duplicata.

**Limite da evidência:** métricas sintéticas/testes não demonstram a frequência de amostras inválidas em Kenseiden, OutRun ou Phantasy Star. A nova métrica não corrige a causa de amostras desconhecidas; torna o comportamento observável.

### 1.3 Correções documentais — integradas

- #1211 integrou a correção do contrato da FIFO para StereoFrame([f64; 2]) e a correção da lista Markdown do runbook.
- #1216 integrou o catálogo de jogos FM e os campos de diagnóstico FM no runbook.
- #1213 integrou o comparador externo de output-code candidate. Seu resultado é evidência de software, não validação da palavra física do DAC.

### 1.4 Ferramentas de benchmark — em validação

- #1216 integrou as métricas FM de fallback ao runbook e o catálogo de jogos compatíveis.
- #1218 acrescenta parser estatístico de logs JSON, percentis p50/p95/p99, frames acima do orçamento, backlog CPAL e baseline de repetibilidade da main com mesmo SHA nos dois lados. A validação de sintaxe PowerShell está pendente.
- O parser diferencia métricas FM indisponíveis no baseline antigo de contagem igual a zero.
- O script A/B padrão continua isolando #1199; o modo same-commit mede a main atual sem alegar A/B de alteração de código.

## 2. Consolidação das PRs de ROM real

A PR #1217 integrou a consolidação sobre a main atual. Inclui Kenseiden Japan e Phantasy Star Japan Rev 1, verifica SHA-256, testa FM não nulo/alinhamento FM+PSG/PCM na sessão de produção, captura escritas F0/F1/F2 antes de reset e adiciona o teste CPAL físico opt-in. CI passou, incluindo compilação Windows do teste CPAL com --no-run. O teste de gameplay e o teste CPAL físico continuam não executados porque dependem de ROMs locais e dispositivo do usuário.

As PRs antigas #1188 e #1193 foram encerradas como substituídas por #1217.

As PRs #1188 e #1193 alteravam os mesmos arquivos:
- `.github/workflows/sms-debugger-runtime.yml`;
- `Cargo.lock`;
- `rust/runtime/sms-runtime/Cargo.toml`;
- `rust/runtime/sms-runtime/tests/fm_in_game_smoke.rs`.

#1188 introduz o harness de gameplay com Kenseiden e Phantasy Star, captura de fontes FM/PSG, mixer/resampler e campanha CPAL. #1193 adiciona ao mesmo harness um probe de até 4.096 instruções para escritas nas portas F0/F1/F2, reseta a máquina após o probe e inclui dependências necessárias.

Essa sobreposição foi resolvida em #1217; não reabrir nem integrar as PRs antigas. O teste continua opt-in e exige ROMs legais locais; nenhuma ROM deve ser commitada.

## 3. Próximas ações ordenadas

1. #1208 passou CI e foi integrada; #1207 foi encerrada.
2. #1213 integrou o comparador output-code; #1216 integrou o catálogo/runbook; #1217 integrou o harness ROM real e compilação Windows do teste CPAL.
3. Concluir a validação e integração do parser A/B #1218 após reaplicá-lo sobre a main pós-#1217.
4. Executar o harness headless com ROMs locais Kenseiden/Phantasy Star e o probe de escritas F0/F1/F2; executar o teste CPAL físico no Windows.
5. Executar Out Run Japan como campanha de desempenho, com hash registrado; não reutilizar o teste Kenseiden como prova para Out Run.
6. Analisar p50/p95/p99, frames acima do orçamento, fallback FM, peak/RMS digital, CPAL underruns/drops e high-water.
7. Medir loopback físico CPAL, backlog/high-water e latência ponta a ponta. A capacidade da FIFO não é latência.
8. Só após localizar o custo dominante avaliar mudanças adicionais no core, batching, AVX2/FMA ou paralelismo. Não criar thread FM que avance o clock do chip.

## 4. Estado de aceite

| Critério | Estado |
|---|---|
| Alocação final por lote removida do caminho CPAL comum | Implementado e CI da PR #1206 aprovado |
| Diagnóstico de amostras FM inválidas | Integrado em #1208; CI aprovado |
| Harness real-game consolidado e integrado | Integrado em #1217; gameplay local pendente |
| Kenseiden / Out Run / Phantasy Star em gameplay | Pendente; ROMs locais necessárias |
| Áudio físico contínuo e sem underruns recorrentes | Pendente |
| Amplitude/volume e clipping calibrados | Pendente |
| Latência interna <10 ms e latência física medida | Pendente |
| 60 FPS sustentados / frame time ≤16,67 ms em jogo real | Pendente |
| Diferenciais de fidelidade e determinismo | Cobertura parcial; execução/aceite pendentes |

**Conclusão:** as primeiras recomendações de baixo risco foram executadas e a alocação por lote no caminho CPAL foi corrigida. A Sprint SUN-FM-01 permanece aberta; nenhuma alegação de 60 FPS, baixa latência ou fidelidade final é feita.
