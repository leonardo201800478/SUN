# SUN V2 — CHRONOS R2 implementation closeout

**Status:** IMPLEMENTAÇÃO EXECUTÁVEL R2-T01–T15 COMPLETA E VALIDADA EM LINUX/WINDOWS; **GATE FORMAL R2 NÃO APROVADO**.
**R1:** APROVADO em 24/11/2026 — v0.3.18, SHA `ac912f342ff303b3fdc554b90d00e85b23d86c76`, PR #1281 / merge `88e105d6495f7f5105502e53666a7b0d29f77572`.
**R2:** issue #1282; PR #1283 permanece Draft, aberta e sem merge.
**R3–R5:** BLOQUEADOS. Nenhum código de produção CHRONOS/V2 autorizado antes de R5.

Este documento fecha o pacote de implementação e evidências disponíveis para a revisão. Não é aprovação formal do gate R2 e não substitui a decisão explícita do responsável.

## 1. Resultado do pacote

O oráculo independente é um artefato Python de verificação, lento e deliberadamente simples. Ele não importa código de produção CHRONOS/V2, não depende de crates Rust e não usa runtime V1. As entradas temporais usam inteiros e `Fraction`; a implementação de referência não escolhe estrutura de agenda nem granularidade de produção.

**Cobertura validada:** 256 testes determinísticos em 13 módulos, incluindo quatro famílias de permutação exaustiva com cinco eventos (120 permutações por família; 480 permutações) e dois mutantes deliberados de fronteira.

| Módulo | Casos | Propriedades |
|---|---:|---|
| `test_temporal_core.py` | 38 | T01–T04 |
| `test_deadlines.py` | 78 | T06 e porções de T07/T14 |
| `test_microsteps.py` | 5 | T14 |
| `test_causal.py` | 13 | T05/T07 |
| `test_partial_order.py` | 6 | T07 |
| `test_transaction.py` | 18 | T08 |
| `test_wait.py` | 11 | T09 |
| `test_irq.py` | 17 | T10 |
| `test_snapshot.py` | 18 | T11 |
| `test_replay.py` | 20 | T12 |
| `test_host.py` | 12 | T13 |
| `test_resource.py` | 10 | T15 |
| `test_atomicity.py` | 10 | T15 |
| **Total** | **256** | **T01–T15** |

A matriz requisito → seção R1 → contrato Cxx → teste e dimensões positivas/negativas/fronteira/igualdade/overflow/permutação está em [R2_TRACEABILITY.md](R2_TRACEABILITY.md).

## 2. Critérios de saída do R2

Os critérios abaixo reproduzem R2_REFERENCE_MODEL.md §9. PASS significa evidência executável/documental no escopo abstrato; PENDING não pode ser convertido em PASS por inferência.

| Critério | Evidência exigida | Estado atual |
|---|---|---|
| **R2-EXIT-01 — T01–T15 reproduzíveis** | Casos positivos, negativos e de fronteira aplicáveis, vinculados ao SHA exato; Linux e Windows do runner Python | **PASS — CI #10785, SHA exato 56d2a85c26fc3f0d7685bb8566af64a445e9b54b** |
| **R2-EXIT-02 — independência do oráculo** | Separação do código V1/produção e da representação candidata; inspeção adversarial da independência | **PARCIAL — isolamento estrutural PASS; revisão independente de correção/circularidade pendente** |
| **R2-EXIT-03 — rastreabilidade R1** | Matriz Txx → seção R1 → Cxx → testes e classes de caso | **PASS — R2_TRACEABILITY.md publicada** |
| **R2-EXIT-04 — UNKNOWN preservado** | Testes de simultaneidade, fase, WAIT, commit e snapshot sem precedência presumida | **PASS — 256/256 Linux e Windows** |
| **R2-EXIT-05 — snapshot/restore/replay/exactly-once** | Cortes suportados/não suportados, ledger e ausência de mutação/repetição | **PASS — 256/256 Linux e Windows** |
| **R2-EXIT-06 — contraexemplo mínimo reproduzível** | Seed, trace, primeira divergência e estado mínimo; redução determinística quando a entrada é gerada | **PASS no escopo implementado — JSON, seeds, traces, primeira divergência e redução de cortes T02; fixtures fixas identificadas pelo teste** |
| **R2-EXIT-07 — revisão independente sem blockers** | Codex revisando SHA final exato, findings respondidos e threads justificadamente encerradas; passada humana adversarial | **PENDENTE — Codex atingiu limite de uso; revisão humana externa ainda necessária** |
| **R2-EXIT-08 — aprovação explícita** | Decisão explícita do responsável após os critérios anteriores | **PENDENTE — NÃO APROVADO** |

### 2.1 Mutação deliberada

`run_mutation_tests.py` aplica cópias temporárias de dois mutantes, sem alterar a árvore de trabalho:

1. `request.at_common < deadline.at_common` → `<=`, verificando que C=D não é classificado como C<D.
2. `IndeterminateCausalOrder` → `LateInput` para C=D sem relação causal, verificando a classe normativa de igualdade UNKNOWN.

Para cada mutante, o runner executa a suíte unittest completa na cópia mutada. O mutante só é **KILLED** quando a suíte inteira retorna falha e o bloco de falha do próprio teste discriminante contém AssertionError; erro de importação ou AssertionError de outro teste não conta. O JSON separa primeira divergência e falha do teste-alvo. Mutante sobrevivente ou âncora não encontrada faz o runner falhar.

### 2.2 Permutações de ordem parcial

`test_partial_order.py` percorre as 120 permutações para cada família:
- cinco eventos simultâneos comutativos;
- cinco eventos simultâneos não comutativos sem ordem (UNKNOWN);
- cinco eventos não comutativos em cadeia causal explícita;
- cinco eventos com raiz, dois irmãos comutativos e descendentes dependentes.

As famílias não atribuem fase elétrica; apenas testam as relações normativas declaradas.

### 2.3 Relatório JSON e reprodução

`run_reference_tests.py`:
- usa seed fixa para embaralhar deterministicamente a ordem de execução dos testes;
- registra HEAD, hashes SHA-256, Python, plataforma, perfil e trace de IDs executados;
- em falha, registra primeiro teste divergente e traceback;
- testes gerados incluem `R2_REPRO` com seed, sequência de operações e estado mínimo; a partição de T02 é reduzida por remoção determinística de cortes;
- inclui resultado de cada mutante;
- emite JSON em stdout e grava arquivo quando `R2_RESULT_PATH` está definido.

As fixtures manuais são reproduzidas pelo ID do teste e pelo estado construído no próprio método. Não há alegação de shrinker genérico para todo o espaço de estados.

## 3. Ambiente e normalização de arquivo

- `.gitattributes` fixa `SUN-V2/verification/**` em LF.
- **CI #10785:** [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/45600632992), HEAD exato `56d2a85c26fc3f0d7685bb8566af64a445e9b54b`.
- Linux: Python 3.14.4 / `Linux-7.0.0-1020-azure-x86_64-with-glibc2.43`; 256/256 PASS, runner duas vezes, `python -m unittest -v`, mutation suite e worktree CLEAN.
- Windows: Python 3.14.4 / `Windows-2025Server-10.0.26100-SP0`; 256/256 PASS, runner duas vezes, `python -m unittest -v`, mutation suite e worktree CLEAN.
- Em ambos: seed de ordem `20261125`; seeds de propriedades T01=`0xC0A05`, T02=0–99; dois mutantes KILLED após executar a suíte completa de 256 testes; `target_test_failed_by_assertion=true`.
- `first_divergence=null` e `minimal_state=null` nas execuções normais aprovadas; os relatórios dos mutantes incluem o primeiro teste divergente, o teste-alvo, excerpt de AssertionError e fixture mínima.
- JSON artifacts: [Linux](https://api.github.com/repos/leonardo201800478/SUN/actions/artifacts/15015027527/zip) (artifact `chronos-r2-linux-45600632992`) e [Windows](https://api.github.com/repos/leonardo201800478/SUN/actions/artifacts/15015127699/zip) (artifact `chronos-r2-windows-45600632992`).
- SHA-256 principal: oráculo `chronos_reference.py` = `3e27b3d387b0af3fa30fd78626e8fd3cc00e1ef289e7b68192c5b3d838a494fb`; runner `run_reference_tests.py` = `de284c4ed3da81573468ea294dfdeefb18adf0e72cb9ae894d2ded8e74b2ed76`; mutation runner = `087beac04e64b376a5e7da36e8e886360dc0d3f3276905846cc81c6a70c7d031`; causal oracle = `43e37b39854efb4b29de5e8a5df0411f116249045fde5a86df915de8febf4720`; deadline oracle = `68a2c8b8315139072cbd75a937fd5699f131847e935c95c2502ee3dd6fedae21`.
- SHA-256 de todos os módulos/testes consta nos dois JSON artifacts e nos logs do CI.
- Change impact classification, documentação patch hygiene, GitHub Actions workflow validation e supervisor Rust quality/unit regression: PASS. Jobs de hardware/runtime específicos não são inferidos como PASS; SKIPPED continua distinto de PASS.

## 4. Limitações explícitas

- **T13:** a fonte de áudio é sintética e inteira; prova somente independência do estado virtual em relação ao pipeline host. Não valida PSG físico, DAC, CPAL ou qualidade sonora.
- **UNKNOWN:** permanece UNKNOWN; nenhuma ordem de inserção, microstep genérico ou ordenação canônica é convertida em fase física.
- **Overflow:** Python `int`/`Fraction` não reproduz overflow de máquina. Capacidade finita/ResourceExhausted é testada; overflow da futura representação finita será comparado em R3, sem antecipar representação numérica.
- **Fidelidade:** PASS significa que o modelo abstrato cumpriu a propriedade executável. Não prova fidelidade de hardware, compatibilidade de ROM ou frame pacing.
- **Ambiente:** jobs SKIPPED não são PASS. Runner Windows não substitui hardware-in-the-loop.

## 5. Histórico de revisão e findings

- A revisão Codex foi solicitada para o SHA exato `56d2a85c26fc3f0d7685bb8566af64a445e9b54b`, mas o conector respondeu **usage limit reached**. Não existe parecer Codex válido desse SHA.
- Os 11 threads P2 pendentes foram respondidos com SHA, testes e CI #10783 e resolvidos; todos os findings P2 conhecidos estão fechados no PR. Isso registra correção e revalidação manual, não substitui revisão independente da PR completa.
- A passada adversarial interna encontrou e corrigiu: (a) ordenação de irmãos comutativos em ordem parcial; (b) ledgers causais pré-aplicados com owner/progresso/dependência/ciclo inconsistentes; (c) imports ausentes no conjunto de permutações; (d) possibilidade de falso positivo no mutation kill report, agora vinculado ao bloco AssertionError do teste-alvo.
- A revisão humana externa ainda deve examinar C=D/A(E)/W, ordem parcial, snapshots, exactly-once, mutation runner, JSON e portabilidade. O Codex indisponível não é considerado PASS.

## 6. Decisão

**Implementação executável R2-T01–T15: COMPLETA PARA VALIDAÇÃO EXTERNA.**
**Gate formal R2: PENDENTE / NÃO APROVADO.**

A sequência obrigatória restante é: obter revisão independente humana do SHA exato `56d2a85c26fc3f0d7685bb8566af64a445e9b54b` sem blockers; registrar a disposição de eventuais findings adicionais; e somente então solicitar aprovação explícita do responsável. CI, matriz, mutação, JSON, Windows e os 11 findings P2 conhecidos já foram executados/respondidos.

Até essa aprovação, PR #1283 permanece Draft, R3–R5 permanecem bloqueados e nenhum código de produção CHRONOS/V2 pode ser iniciado.
