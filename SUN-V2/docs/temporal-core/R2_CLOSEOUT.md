# SUN V2 — CHRONOS R2 implementation closeout

**Status:** implementação R2-T01–T15 revisada independentemente; parecer do responsável sem blocker funcional; dois P2 documentais corrigidos; aprovação explícita do responsável registrada. Validação final do pacote documental por CI pendente.
**R1:** APROVADO em 24/11/2026 — v0.3.18, SHA `ac912f342ff303b3fdc554b90d00e85b23d86c76`, PR #1281 / merge `88e105d6495f7f5105502e53666a7b0d29f77572`.
**R2:** issue #1282; PR #1283 permanece aberta, em revisão/fechamento documental, sem merge.
**R3:** protótipos descartáveis autorizados após aprovação formal R2, CI final verde e merge da PR. **R4–R5:** bloqueados. Nenhum código de produção CHRONOS/V2 autorizado antes de R5. **R4–R5:** bloqueados. Nenhum código de produção CHRONOS/V2 autorizado antes de R5.

Este documento registra o fechamento técnico do R2, o parecer independente do responsável e sua aprovação explícita. A validação CI do HEAD documental final permanece como último controle antes do merge.

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
| **R2-EXIT-01 — T01–T15 reproduzíveis** | Casos positivos, negativos e de fronteira aplicáveis, vinculados ao SHA exato; Linux e Windows do runner Python | **PASS — CI #10790, SHA exato `ad8cbbe50a82334b8ee873351dda65b26459e239`; 256/256 em Linux e Windows, mutation PASS.** |
| **R2-EXIT-02 — independência do oráculo** | Separação do código V1/produção e da representação candidata; inspeção adversarial da independência | **PASS no escopo da revisão: parecer independente do responsável em `ad8cbbe50a82334b8ee873351dda65b26459e239`, sem findings funcionais bloqueantes nesta passada.** |
| **R2-EXIT-03 — rastreabilidade R1** | Matriz Txx → seção R1 → Cxx → testes e classes de caso | **PASS — R2_TRACEABILITY.md publicada** |
| **R2-EXIT-04 — UNKNOWN preservado** | Testes de simultaneidade, fase, WAIT, commit e snapshot sem precedência presumida | **PASS — 256/256 Linux e Windows** |
| **R2-EXIT-05 — snapshot/restore/replay/exactly-once** | Cortes suportados/não suportados, ledger e ausência de mutação/repetição | **PASS — 256/256 Linux e Windows** |
| **R2-EXIT-06 — contraexemplo mínimo reproduzível** | Seed, trace, primeira divergência e estado mínimo; redução determinística quando a entrada é gerada | **PASS no escopo implementado — JSON, seeds, traces, primeira divergência e redução de cortes T02; fixtures fixas identificadas pelo teste** |
| **R2-EXIT-07 — revisão independente sem blockers** | Codex revisando SHA final exato, findings respondidos e threads justificadamente encerradas; passada humana adversarial | **PASS — parecer independente do responsável publicado no PR #1283 em 25/11/2026 às 18:55 UTC, revisando exatamente `ad8cbbe50a82334b8ee873351dda65b26459e239`; nenhum finding funcional bloqueante. Os dois P2 documentais foram tratados nesta revisão; CI do novo HEAD documental pendente.** |
| **R2-EXIT-08 — aprovação explícita** | Decisão explícita do responsável após os critérios anteriores | **PASS — aprovação explícita concedida pelo responsável nesta conversa em 25/11/2026, após o parecer independente; efetivação vinculada à validação CI do SHA documental final.** |

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
- **CI #10790:** [SUCCESS](https://github.com/leonardo201800478/SUN/actions/runs/45612059123), HEAD de código revisado `ad8cbbe50a82334b8ee873351dda65b26459e239`; Linux e Windows 256/256, mutation PASS (dois mutantes KILLED), worktrees CLEAN, higiene documental e Actionlint PASS. [JSON Linux](https://github.com/leonardo201800478/SUN/actions/runs/45612059123/artifacts/15019892683) · [JSON Windows](https://github.com/leonardo201800478/SUN/actions/runs/45612059123/artifacts/15020322807).
- Linux: Python 3.14.4 / `Linux-7.0.0-1020-azure-x86_64-with-glibc2.43`; 256/256 PASS, runner duas vezes, `python -m unittest -v`, mutation suite e worktree CLEAN.
- Windows: Python 3.14.4 / `Windows-2025Server-10.0.26100-SP0`; 256/256 PASS, runner duas vezes, `python -m unittest -v`, mutation suite e worktree CLEAN.
- Em ambos: seed de ordem `20261125`; seeds de propriedades T01=`0xC0A05`, T02=0–99; dois mutantes KILLED após executar a suíte completa de 256 testes; `target_test_failed_by_assertion=true`.
- `first_divergence=null` e `minimal_state=null` nas execuções normais aprovadas; os relatórios dos mutantes incluem o primeiro teste divergente, o teste-alvo, excerpt de AssertionError e fixture mínima.
- JSON artifacts: [Linux](https://api.github.com/repos/leonardo201800478/SUN/actions/artifacts/15015027527/zip) (artifact `chronos-r2-linux-45600632992`) e [Windows](https://api.github.com/repos/leonardo201800478/SUN/actions/artifacts/15015127699/zip) (artifact `chronos-r2-windows-45600632992`).
- SHA-256 dos artefatos do HEAD `ad8cbbe50a82334b8ee873351dda65b26459e239` está registrado nos JSON artifacts e logs do CI #10790. O SHA-256 atualizado de `run_mutation_tests.py` é `6cc2fc6fd0a972aae881fcb622c5136f6db30fcc061f7f8ec5755d88b3a9b5a9`; os hashes do pacote anterior (CI #10785) são históricos e não representam esse HEAD.
- SHA-256 de todos os módulos/testes consta nos dois JSON artifacts e nos logs do CI.
- Change impact classification, documentação patch hygiene, GitHub Actions workflow validation e supervisor Rust quality/unit regression: PASS. Jobs de hardware/runtime específicos não são inferidos como PASS; SKIPPED continua distinto de PASS.

## 4. Limitações explícitas

- **T13:** a fonte de áudio é sintética e inteira; prova somente independência do estado virtual em relação ao pipeline host. Não valida PSG físico, DAC, CPAL ou qualidade sonora.
- **UNKNOWN:** permanece UNKNOWN; nenhuma ordem de inserção, microstep genérico ou ordenação canônica é convertida em fase física.
- **Overflow:** Python `int`/`Fraction` não reproduz overflow de máquina. Capacidade finita/ResourceExhausted é testada; overflow da futura representação finita será comparado em R3, sem antecipar representação numérica.
- **Fidelidade:** PASS significa que o modelo abstrato cumpriu a propriedade executável. Não prova fidelidade de hardware, compatibilidade de ROM ou frame pacing.
- **Ambiente:** jobs SKIPPED não são PASS. Runner Windows não substitui hardware-in-the-loop.

## 5. Histórico de revisão e findings

- A revisão independente do responsável foi registrada como parecer na PR #1283 em 25/11/2026 às 18:55 UTC, ancorada no HEAD exato `ad8cbbe50a82334b8ee873351dda65b26459e239`. Não apontou finding funcional bloqueante nesta passada. Identificou dois P2 documentais descritos abaixo. O parecer humano foi efetivamente fornecido; revisão automática Codex não foi obtida por limite de uso e não é tratada como evidência.
- Os 11 threads P2 pendentes foram respondidos com SHA, testes e CI #10783 e resolvidos; todos os findings P2 conhecidos estão fechados no PR. Isso registra correção e revalidação manual, não substitui revisão independente da PR completa.
- A passada adversarial interna encontrou e corrigiu: (a) ordenação de irmãos comutativos em ordem parcial; (b) ledgers causais pré-aplicados com owner/progresso/dependência/ciclo inconsistentes; (c) imports ausentes no conjunto de permutações; (d) possibilidade de falso positivo no mutation kill report, agora vinculado ao bloco AssertionError do teste-alvo.
- **P2-1 (R2_REFERENCE_MODEL.md):** cabeçalho histórico dizia T01–T04 parciais. Corrigido no commit documental `413a81d3bb6ec162e133a2ec78e9d78f05d0785b`, agora declara T01–T15 concluídos para revisão externa e gate formal não aprovado.
- **P2-2 (este closeout):** precisava ligar critérios e artefatos ao HEAD que o parecer revisou, registrar os hashes correntes, a evidência do CI #10790 e o próprio parecer. Esta atualização faz essa vinculação. A revalidação do novo SHA documental é obrigatória antes da efetivação do gate.



## 5.1 Disposição do parecer independente — 25/11/2026

O parecer do responsável no PR #1283 avaliou o HEAD `ad8cbbe50a82334b8ee873351dda65b26459e239`: nenhuma falha funcional bloqueante identificada; dois P2 documentais:

- **P2-1:** cabeçalho de `R2_REFERENCE_MODEL.md` desatualizado. Corrigido no commit `413a81d3bb6ec162e133a2ec78e9d78f05d0785b`.
- **P2-2:** o closeout vinculava os critérios a SHA/artefatos anteriores. Corrigido nesta revisão do closeout: identifica o SHA do parecer, CI #10790, artifacts Linux/Windows e SHA-256 corrente do mutation runner. O próprio commit documental será validado por CI.

O parecer não autorizou merge antes dessas correções e revalidação; a aprovação explícita do responsável foi concedida depois do parecer nesta conversa.

## 6. Decisão

**Implementação executável R2-T01–T15: COMPLETA E REVISADA.**
**Gate formal R2: APROVADO pelo responsável em 25/11/2026; publicação na `main` e início de R3 dependem da confirmação de CI verde no HEAD final e do merge da PR.**

Os dois P2 documentais foram corrigidos: cabeçalho R2_REFERENCE_MODEL atualizado no commit `413a81d3bb6ec162e133a2ec78e9d78f05d0785b`; closeout atualizado nos commits `e1301d3f7c47fde9e0221c9d3803cd7f60c0ba29` e sequente. O parecer independente não identificou blockers funcionais. CI #10790 valida o código do HEAD revisado `ad8cbbe`; a validação de higiene documental e dos commits de atualização será realizada pelo CI acionado pela atualização da PR.

Após CI verde do SHA documental final, atualizar issue/PR para R2 APROVADO, publicar PR #1283 na `main` e abrir o trabalho R3 em branch própria. R3 limita-se a protótipos descartáveis e benchmarks comparáveis; nenhuma implementação de produção CHRONOS/V2 antes de R5.
