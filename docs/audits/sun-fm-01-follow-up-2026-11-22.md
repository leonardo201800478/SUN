# SUN-FM-01 — Execução das recomendações da auditoria

**Data:** 2026-11-22
**Base de trabalho:** main pós-merge #1206, #1208, #1213, #1216, #1217, #1219 e #1220, `902e47cd63736ab2cadbcaa051cfc2d55b67c0ad`
**Estado:** acompanhamento em andamento; sem aprovação de gameplay ou conclusão da sprint.

## Checkpoint após merge da PR #1223 — 2026-11-22

- PR #1223 foi integrada à `main` por squash: `ac319d4cbb93f624c7830d628ccb36c9b5b33fbb`.
- O serviço PG serial do perfil Japanese SMS agora instala explicitamente o candidato de reset de software do LFSR `(register = 0, next_slot_index = 0)`. O código não afirma que esse seja o seed físico de power-on.
- A regressão `japanese_fm_rhythm_mode_has_known_noise_and_restores_deterministically` ativa as cinco vozes, verifica PCM digital conhecido/não silencioso e compara execução contínua com snapshot/restore.
- CI #10316 e SMS Debugger Runtime #1886 concluíram com sucesso na PR. A cobertura é sintética/core/runtime; não executa ROMs comerciais nem dispositivo CPAL físico.
- Os resultados locais abaixo foram obtidos no commit antigo `d6003f6dc`, antes do merge #1223. Eles permanecem evidência histórica válida daquele build, mas não medem o comportamento do novo HEAD. Reexecutar Kenseiden e Phantasy Star no checkout atualizado da `main` antes de atribuir qualquer mudança de áudio à correção.
- HEAD confirmado da `main`: `ac319d4cbb93f624c7830d628ccb36c9b5b33fbb`.


## Resultado da primeira execução local de ROM real — 2026-11-22

O usuário executou o harness headless no commit `d6003f6dc` com Kenseiden (Japan) e Phantasy Star (Japan) Rev 1, ambos com SHA-256 correspondente aos fixtures.

- Kenseiden: 497.023 amostras FM; 405.658 não nulas; pico digital `0.06274286`; 624 blocos dual-source alinhados; 1.801 blocos PCM. Isso demonstra atividade numérica na rota digital de compatibilidade, **não** qualidade musical, fidelidade de timbre, volume acústico ou aprovação funcional.
- Phantasy Star: a fonte FM permaneceu silenciosa durante 1.800 frames e a asserção do teste falhou. O contador baseado no trace essencial informou 30.570 eventos descartados, portanto os zeros reportados para F0/F1/F2 não eram evidência confiável de ausência de escritas.
- A página de compatibilidade interna não deve excluir o lançamento japonês de Phantasy Star: o SMS Power lista explicitamente FM apenas na versão japonesa do jogo. As versões ocidentais foram localizadas sem o código FM. Fontes: https://www.smspower.org/Tags/FM e https://www.smspower.org/Games/PhantasyStar-SMS.

Decisão de aceite: **Kenseiden e Phantasy Star permanecem NÃO APROVADOS**. `fm_nonzero_samples > 0`, peak/RMS digital e PCM não silencioso são verificações de atividade, não critérios suficientes de qualidade.

### Instrumentação adicionada nesta linha de trabalho

- Contadores monotônicos F0/F1/F2 e por registrador, atualizados no caminho de conclusão da escrita do barramento. Não dependem do trace circular; são diagnósticos e não fazem parte do estado emulado/salvo.
- Captura opcional, por `SEGA_SMS_FM_TEST_WAV_DIR`, de quatro sinais por ROM: FM pré-mixer, PSG pré-mixer, blocos `single-source` (fallback/PSG-only no perfil FM ativo) e PCM mono final após o `NativeAudioSession`.
- WAV IEEE float32 mono a 48 kHz, sem normalização e sem clipping deliberado; relatórios incluem quantidade, não nulas, não finitas, valores com módulo >= 1, pico, RMS e média.
- O teste permanece um smoke/diagnóstico. A aprovação exige comparação com referência Ares/Genesis Plus GX para a mesma ROM, revisão, cena e perfil, além de avaliação auditiva de música, timbre, envelope, dinâmica, ruído, clipping e continuidade.
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

### 1.4 Ferramentas de benchmark — integradas

- #1216 integrou as métricas FM de fallback ao runbook e o catálogo de jogos compatíveis.
- #1220 integrou parser estatístico de logs JSON com p50/p95/p99, média, desvio, frames acima do orçamento, cadência do worker e backlog CPAL.
- O parser diferencia métricas FM indisponíveis no baseline antigo de contagem igual a zero e foi exercitado por self-test com quatro execuções sintéticas no Windows CI.
- O A/B padrão continua isolando #1199; o modo same-commit mede a main atual sem alegar A/B de alteração de código.
- O relatório não chama cadência do worker de scanout nem backlog FIFO de latência ponta a ponta.

## 2. Consolidação das PRs de ROM real

A PR #1217 integrou a consolidação sobre a main atual. Inclui Kenseiden Japan e Phantasy Star Japan Rev 1, verifica SHA-256, testa FM não nulo/alinhamento FM+PSG/PCM na sessão de produção e adiciona o teste CPAL físico opt-in. A captura inicial de F0/F1/F2 era um probe pré-execução limitado e sujeito a sobrescrita do trace; esta branch substitui essa abordagem por contadores diretos no caminho de conclusão do barramento durante a execução real. CI passou, incluindo compilação Windows do teste CPAL com --no-run. O teste de gameplay e o teste CPAL físico continuam não executados porque dependem de ROMs locais e dispositivo do usuário.

As PRs antigas #1188 e #1193 foram encerradas como substituídas por #1217.

As PRs #1188 e #1193 alteravam os mesmos arquivos:
- `.github/workflows/sms-debugger-runtime.yml`;
- `Cargo.lock`;
- `rust/runtime/sms-runtime/Cargo.toml`;
- `rust/runtime/sms-runtime/tests/fm_in_game_smoke.rs`.

#1188 introduz o harness de gameplay com Kenseiden e Phantasy Star, captura de fontes FM/PSG, mixer/resampler e campanha CPAL. #1193 adiciona ao mesmo harness um probe de até 4.096 instruções para escritas nas portas F0/F1/F2, reseta a máquina após o probe e inclui dependências necessárias.

Essa sobreposição foi resolvida em #1217; não reabrir nem integrar as PRs antigas. O teste continua opt-in e exige ROMs legais locais; nenhuma ROM deve ser commitada.

## 3. Pendências após as entregas integradas

Entregas integradas e validadas por CI: #1208 (diagnóstico), #1213 (comparador), #1216 (catálogo/runbook), #1217 (harness), #1220 (parser/self-test) e #1223 (Rhythm Mode reset candidate + regressão snapshot/restore).

1. Executar o harness headless com ROMs locais Kenseiden/Phantasy Star e o probe de escritas F0/F1/F2; executar o teste CPAL físico no Windows.
2. Executar Out Run Japan como campanha de desempenho, com hash registrado; não reutilizar o teste Kenseiden como prova para Out Run.
3. Analisar p50/p95/p99, frames acima do orçamento, fallback FM, peak/RMS digital, CPAL underruns/drops e high-water.
4. Medir loopback físico CPAL, backlog/high-water e latência ponta a ponta. A capacidade da FIFO não é latência.
5. Só após localizar o custo dominante avaliar mudanças adicionais no core, batching, AVX2/FMA ou paralelismo. Não criar thread FM que avance o clock do chip.

## 4. Execução local pendente — Windows

O CI validou compilação e ferramentas, mas não possui ROMs comerciais nem dispositivo de áudio físico. Para fechar a validação de software com ROM real, no checkout atualizado da main e com as ROMs obtidas legalmente:

```powershell
$env:SEGA_SMS_FM_TEST_ROM_DIR = "D:\ROMs\SMS-FM"
$env:SEGA_SMS_FM_TEST_REGION = "japan"
$env:SEGA_SMS_FM_TEST_SYSTEM = "sms1"
$env:SEGA_SMS_FM_TEST_FRAMES = "1800"
$env:SEGA_SMS_FM_TEST_WAV_DIR = "D:\\SUN-FM-Captures" # opcional: FM, PSG e PCM final em WAV float

cargo test --release --locked -p sega-sms-runtime --test fm_in_game_smoke real_japanese_sms_roms_emit_non_silent_fm_through_the_native_mixer -- --ignored --nocapture
cargo test --release --locked -p sega-sms-runtime --test fm_in_game_smoke real_japanese_sms_roms_play_fm_through_cpal_and_report_underruns -- --ignored --nocapture
```

O diretório deve conter Kenseiden (Japan), SHA-256 `7a9c056edfb1b973482430a41074b0b0a076ee4591d5df8f19007e7058cadc93`, e Phantasy Star (Japan) Rev 1, SHA-256 `55ba15a1ea5c7db883a7a4c850bea2b1024185605964916ddd04a645d7ddd2b6`. O teste verifica os hashes; não renomeie uma ROM diferente para contornar a verificação. O segundo comando abre o dispositivo CPAL padrão e reproduz o áudio; deve ser executado no Windows com dispositivo de saída funcional.

Para medir a main atual em Kenseiden, Out Run (Japan) e Phantasy Star (Japan Rev 1), use o modo same-commit em campanhas separadas. Antes, habilite a coleta diagnóstica pelo menu DEBUG (consentimento de logs/métricas), sem ativar Modo Debugger, e configure SMS1, região Japão, Japanese SMS integrado e FM On:

```powershell
git fetch origin main
$mainSha = (git rev-parse origin/main).Trim()
.\scripts\benchmark\ym2413-real-game-ab.ps1 -RomPath "D:\ROMs\Kenseiden (Japan).sms" -BaselineSha $mainSha -CandidateSha $mainSha -CaptureSeconds 60 -WarmupSeconds 15
```

Repita trocando `-RomPath` para a ROM japonesa de Out Run e Phantasy Star. O script registra o SHA-256 de cada ROM e solicita a mesma cena em cada execução. Para a campanha final, aumente CaptureSeconds para 180. Após cada ABBA, use o caminho de saída informado pelo script:

```powershell
.\scripts\benchmark\summarize-ym2413-real-game-ab.ps1 -BenchmarkDirectory "<diretorio-da-execucao>"
```

O resumo gera summary.json, summary.csv e comparison.csv. O modo same-commit mede repetibilidade/cadência da main, não ganho de uma mudança de código. A campanha padrão continua isolando #1199. Para volume/clipping e latência ponta a ponta, ainda será necessária captura loopback física; peak/RMS digital e high-water FIFO não substituem essa medição.

## 5. Estado de aceite

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
