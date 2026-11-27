# SUN V2 — proposta de tratamento das lacunas R1/R2 identificadas em R3

**Classificação:** proposta técnica de revisão; não altera R1 nem R2
**Estado:** aberta para revisão independente
**R1 aprovado:** v0.3.18, conteúdo SHA `ac912f342ff303b3fdc554b90d00e85b23d86c76`, merge `88e105d6495f7f5105502e53666a7b0d29f77572`
**R2 aprovado:** PR #1283, merge SHA `18b4352e16cf6ab275c4923a30acc3124754a1b0`
**P-A inicial revisado:** `977641816c7396ee96c6f916efea4ee736042e24`
**P-A correções em validação:** `827e2da1dc87c2b4c98ef9b1171313f14c35ecf5`

## 1. Objetivo e limites

Registrar duas lacunas observadas durante a revisão independente do protótipo P-A (PR #1284), preservar os artefatos R1/R2 aprovados e propor uma forma controlada de tratar divergências entre o contrato normativo, o oráculo aprovado e o protótipo experimental.

Este documento **não**:
- altera o contrato R1;
- altera, reescreve ou invalida automaticamente o modelo R2;
- declara que R2 é globalmente incorreto;
- autoriza código de produção CHRONOS/V2;
- promove R3, R4 ou R5.

## 2. Finding W07 — progressão entre níveis de microstep

### FACT — contrato aprovado

R1 §6, item 1, proíbe observar um evento posterior antes de um evento causalmente anterior. R1 §16.6 define microstep/delta-cycle como ordem lógica de software no mesmo instante, restringe a sequência de microsteps a dependências causais já estabelecidas ou efeitos comutativos e exige regra explícita para eventos criados durante despacho.

### FACT — lacuna observada no R2

No R2 aprovado:
- `microstep_reference.py` (blob SHA `5e00999c74cee52012124dff7615ba50cd8348d5`) seleciona a obrigação pelo identificador e verifica conflitos no nível selecionado;
- `test_microsteps.py` (blob SHA `b0ba8b59ac6b9fd4a3195ab40f0c11742da786d0`) cobre emissão de filho no próximo microstep, mas não o caso em que um irmão comutativo do nível anterior continua pendente quando o filho é solicitado.

O R2 não possui, portanto, teste discriminante para a tentativa de processar C no nível 1 enquanto B ainda está pendente no nível 0.

### OBSERVATION — correção experimental P-A

P-A passa a bloquear a seleção de uma obrigação quando existe obrigação pendente em nível lógico anterior. Antes de bloquear, mantém `IndeterminateCausalOrder` se o nível anterior contiver conflito simultâneo não comutativo sem relação demonstrada.

O teste adversarial:
1. cria A e B comutativas no nível 0;
2. processa A e emite C no nível 1;
3. tenta processar C enquanto B permanece pendente;
4. verifica `Blocked`, identidade do estado preservada e B ainda pendente;
5. processa B;
6. processa C e conclui o ciclo.

O teste também registra explicitamente que o R2 aprovado aceita o passo 3. Essa divergência é intencionalmente excluída da comparação diferencial positiva; não se altera a expectativa normativa para forçar equivalência.

### HYPOTHESIS / INFERENCE — interpretação operacional

A barreira de menor nível é uma interpretação experimental conservadora da progressão lógica de microsteps. Ela não declara precedência física entre dispositivos nem usa ordem de inserção. A interpretação deve ser avaliada na revisão independente R3/R4; se a revisão entender que R1 não determina o fechamento do nível anterior, a ambiguidade deve ser resolvida formalmente antes de selecionar o modelo.

## 3. Finding W06 — comparabilidade de evidências de fase

### FACT — contrato aprovado

R1 §6 distingue instantes diferentes, fases comprovadamente diferentes, mesma fase e relação desconhecida. Fases físicas dependem de evidência do perfil ou decisão validada; sequência de inserção não prova precedência elétrica. Efeitos não comutativos sem relação de fase conhecida permanecem não resolvidos.

### FACT — lacuna observada no R2

No R2 aprovado:
- `deadline_reference.py` (blob SHA `1b9e5e3b80425aadf8c5d98a38a93133fc1ea986`) exige phase order e phase evidence não ausentes e ordens distintas, mas não exige uma base de comparação estruturada;
- `test_deadlines.py` (blob SHA `03054b12cfbcc8c321e70a0f55e4b118c0b05565`) testa ordens distintas usando uma única string sintética (`synthetic-test-fixture`), sem testar perfis/bases distintos.

Assim, o oráculo pode aceitar duas ordens numericamente distintas sem demonstrar que elas compartilham perfil, revisão, coordenada, unidade e escala comparáveis.

### OBSERVATION — correção experimental P-A

P-A introduz `APhaseBasis`, descritor tipado com:
- perfil e revisão;
- coordenada e unidade;
- numerador e denominador da escala;
- identidade e revisão da base;
- identificador e SHA-256 do registro de validação.

Para eventos não comutativos no mesmo microstep, P-A exige ordens distintas, evidências presentes e descritores de base estruturalmente válidos e iguais. Base ausente ou bases diferentes produzem `IndeterminateCausalOrder`, sem reivindicar deadlines e sem fechar watermark. Descritor malformado é rejeitado na publicação, sem mutação.

Os testes positivos usam fixtures sintéticas para exercitar a mecânica de comparação. O digest sintético não é prova de evidência física, nem significa que um perfil SMS real tenha sido validado. Sem registro externo real, a fase física permanece UNKNOWN.

### HYPOTHESIS / INFERENCE — limite da representação

A igualdade de descritores tipados estabelece que os eventos declaram a mesma base de comparação; a validação estrutural verifica a forma e o digest declarado. A existência, autoridade e conteúdo do registro referenciado continuam sendo uma fronteira externa que precisa ser auditada. A representação não deve ser promovida a evidência física até que o registro de perfil correspondente seja localizado e validado.

## 4. Tratamento formal proposto para R2

1. Preservar o merge SHA R2 `18b4352e16cf6ab275c4923a30acc3124754a1b0` e todos os blobs aprovados.
2. Não editar `SUN-V2/verification/r2_reference/` como parte da correção P-A.
3. Manter os testes de divergência P-A como testes normativos independentes, explicitando que não são equivalência diferencial.
4. Anexar os casos mínimos W07/W06 a uma proposta formal de revisão do modelo R2.
5. Solicitar revisão independente da interpretação de R1 e do impacto sobre os demais contratos.
6. Somente após decisão explícita, abrir revisão de R2 com novos testes, closeout, rastreabilidade, mutation checks e CI em ambos os sistemas operacionais.
7. Até essa decisão, tratar R2 como oráculo aprovado para o escopo coberto, mas não como prova suficiente para os dois cenários recém-identificados.

## 5. Evidência da correção P-A

A validação desta revisão está associada ao SHA `827e2da1dc87c2b4c98ef9b1171313f14c35ecf5` e CI #10930.

Os testes P-A foram ampliados para 125 casos por sistema operacional; os testes independentes R2 permanecem em 256 por sistema operacional. A execução dos benchmarks e a verificação cruzada dos artefatos Linux/Windows são registradas em `benchmarks/r3/P_A_BASELINE_RESULTS.md` após o fechamento da execução.

## 6. Decisão pendente

**Proposta:** manter a correção experimental P-A, preservar R2 imutável e submeter as duas lacunas a revisão independente. Não declarar R2 reprovado nem R3 fechado por esta proposta.

R4/R5 continuam pendentes. Nenhum código de produção CHRONOS/V2 está autorizado antes de R5.


## 7. Resultado da revisão adversarial do P-A (26/11/2026)

A revisão do HEAD `09f71c8aa059c44855c358b614c965f92900a173` mantém a proposta aberta e acrescenta as seguintes qualificações:

- **W07 permanece BLOCKED.** O protótipo usa `commutative: bool` por obrigação e bloqueia o filho em microstep posterior até resolver obrigações anteriores. No caso B (nível 0) / C (nível 1), isso impõe B<C sem demonstrar uma dependência causal B→C nem a comutatividade do par B/C. R1 §16.6.2 não permite que a mera numeração de microsteps crie precedência. A regra atual é hipótese experimental, não conformidade demonstrada.
- **W06 permanece limitado a comparação sintética.** A igualdade estrutural de `APhaseBasis` impede comparar descritores diferentes, mas o identificador e o digest do registro de validação não são resolvidos nem autenticados contra um registro externo. A evidência física de fase SMS continua UNKNOWN.
- **O corpus de P-A ainda não é um trace canônico reexecutável completo.** O emissor registra IDs de fixtures, índices, base e digests dos cortes, mas não serializa entradas, operações e estados completos. Os 1.475 cortes e o digest Linux/Windows idêntico são evidência útil, porém insuficiente para alimentar P-D/P-E com as mesmas sequências e realizar comparação independente completa.

**Decisão de revisão:** preservar R1/R2 e a implementação experimental atual; não declarar W06/W07 fechados; não encerrar P-A; não iniciar P-D/P-E até resolver os blockers e completar o corpus (ou revisar formalmente o requisito de equivalência, sem enfraquecer R1). Nenhuma alteração de semântica normativa é aprovada por este registro.

## 8. W07 — matriz discriminante mínima para revisão

A inspeção direta de `p_a_microsteps.py`, `test_p_a_microsteps.py` e do oráculo R2 confirma que o cenário precisa distinguir **causalidade de microstep** de **ordenação entre irmãos independentes**:

- A e B começam no microstep 0 e são declarados comutativos entre si.
- Ao resolver A, o proprietário emite C no microstep 1; a dependência A→C está estabelecida.
- B continua pendente no microstep 0. O fato de B ser comutativo com A não demonstra que B seja comutativo com C.

A regra atual de P-A bloqueia C, processa B e então C. Isso escolhe B<C sem declarar B→C nem provar comutatividade(B,C). A alternativa de simplesmente permitir C antes de B também não é válida quando a relação B/C é desconhecida e os efeitos podem ser não comutativos. Portanto, para o caso B/C sem relação conhecida, o resultado conservador exigido pelo contrato é **IndeterminateCausalOrder**, sem mutação e sem fechar o corte.

Matriz mínima para uma futura revisão experimental, sem alterar R1/R2:

| Relação declarada entre B e C | Resultado esperado |
|---|---|
| B→C causalmente estabelecida | B deve resolver antes de C |
| C→B causalmente estabelecida | C deve resolver antes de B |
| B e C comprovadamente comutativos | Qualquer ordem determinística é permitida, sem alterar o resultado observável |
| Relação desconhecida e efeitos potencialmente não comutativos | IndeterminateCausalOrder; preservar estado e watermark |

A relação deve ser explícita por par (ou derivada de uma prova formalmente definida); o booleano unário `commutative` de cada obrigação não prova comutatividade entre duas obrigações distintas. A herança transitiva de comutatividade também não deve ser presumida: commutative(A,B) e causal(A,C) não implicam commutative(B,C).

**Gate de revisão:** antes de alterar P-A, submeter esta matriz a revisão independente contra R1 §§6 e 16.6 e avaliar o gap correspondente no R2 aprovado. Qualquer mudança no oráculo exige processo separado de revisão R2, novos testes discriminantes, rastreabilidade e aprovação explícita. Não alterar o R2 aprovado dentro da PR #1284.


## 8. Evidência replayável W06 — divergência P-A/R2 (27/11/2026)

A fixture `SUN-V2/benchmarks/r3/test_w06_replayable_fixture.py` registra o caso mínimo de comparabilidade de bases de fase:

- Dois deadlines não comutativos são publicados no mesmo instante comum.
- Cada deadline declara `phase_order` distinto e `phase_evidence` presente, mas as `APhaseBasis` são sintéticas e distintas (perfil/base diferentes).
- No corte de fechamento do watermark, P-A retorna `IndeterminateCausalOrder`, mantém os dois deadlines em `Scheduled`, não reivindica nenhum e não avança o watermark.
- O oráculo R2 retorna `Accepted`, reivindica `phase-a` e `phase-b` pela ordem numérica e avança o watermark.

A diferença é registrada como **divergência intencional** no corpus replayável, não removida para obter igualdade. Os dois publishes anteriores concordam; o primeiro corte divergente é a operação `close_watermark` (sequência 2). Os adapters preservam os descritores `phase_basis` como sidecar declarativo da fixture; R2 não possui nem interpreta esse campo.

**Validação:** CI #11050, run `46004316797`, SHA `a2e9f68bb6ebedabe9750eb227d2e94fe82408fe`; 169 testes R3 PASS em Linux e Windows. Corpus replayável: 24 fixtures/131 operações-cortes, duas divergências (W06/W07), digest `07551ec877a58597180470045ad24bd51444124d469b2dc2efdcc4e3d9dcba7e`; SHA-256 JSON idêntico Linux/Windows `fbb0940c3d8459763803193b67fd2d974241d66e3a50a98d560afd82a71c87ce`.

**Limite de evidência:** os IDs de validação e hashes da fixture são placeholders sintéticos; não apontam para registros externos nem trust anchors. A divergência mostra apenas que os modelos aplicam critérios diferentes de comparabilidade. Ela não estabelece qual ordenação é correta no hardware, nem autentica fase física SMS. Evidência externa/física W06 permanece **UNKNOWN**.

**Disposição:** manter R2 aprovado e imutável; preservar a divergência e encaminhar a diferença para revisão independente. Nenhuma alteração normativa R1/R2, closeout P-A, P-D/P-E, R4/R5 ou código de produção é autorizada por este aditivo.
