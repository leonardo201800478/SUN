# YM2413 — auditoria da ordem de consumo do contador EG no modelo Andete 2017

**Data:** 2026-11-19
**Base Rust:** `main` após PR #1053 e atualizações FM-1/FM-3.
**Escopo:** ordem de incremento/consumo do contador global e sequência de chamadas PG → EG → operador no modelo `results/20170126/model5.cc`.
**Status:** caracterização de referência de software; não certifica a ordem física do silício.

## 1. Fontes examinadas

1. Andete, `results/20170126/model5.cc` — modelo C++ completo do operador e do ciclo YM2413:
   https://github.com/andete/ym2413/blob/master/results/20170126/model5.cc
2. Andete, `results/20170126/attack.txt` — medições de ataque, hipóteses e ressalvas do autor:
   https://github.com/andete/ym2413/blob/master/results/20170126/attack.txt
3. Andete, notas de 2015-03-20 — medições de decay/release e contador global:
   https://www.smspower.org/Development/YM2413ReverseEngineeringNotes2015-03-20
4. Implementação Rust: `native/ym2413/src/envelope.rs`, `phase.rs`, `synthesis.rs` e `lib.rs`.

O texto `attack.txt` declara que o modelo atualizado ainda precisava de verificação adicional. As conclusões abaixo descrevem exatamente a ordem do modelo de software, não uma captura independente de todos os sinais internos do chip.

## 2. Ordem observável em `model5.cc`

### 2.1 Incremento global

`YM2413::calc()` executa `++counter` no início da chamada. Depois:
- atualiza o estado do AM quando o contador incrementado é múltiplo de 64;
- percorre os canais;
- cada canal calcula primeiro o modulator e depois o carrier.

Consequentemente, no modelo, o valor de `counter` consumido por PG/EG/PM em uma chamada de saída é o valor **pós-incremento**. O mesmo valor permanece estável durante o processamento de todos os operadores/canais dessa chamada. O contador não é incrementado uma vez por operador nem por slot.

Isso é compatível com a descrição de 2015 de um contador global compartilhado e incrementado uma vez por amostra. Não prova que o contador físico tenha origem zero, a mesma largura, o mesmo wrap ou a mesma ordem de observação após reset.

### 2.2 Ordem por operador

`Operator::calcOp()` executa, nesta ordem:

1. `calcPhase()`: atualiza o acumulador PG e retorna o índice de fase resultante;
2. `calcEnv()`: atualiza o EG e pode emitir a transição DAMP;
3. calcula a saída usando a fase já calculada e o nível EG atualizado.

O modelo contém comentários TODO sobre a relação de uso da fase e do envelope na mesma amostra. Portanto, a ordem acima é a sequência implementada pelo modelo, mas não deve ser elevada isoladamente a prova de latch/subfase física.

### 2.3 Ordem modulator/carrier e DAMP

`Channel::calcChan()` chama `mod.calcMod()` antes de `car.calcCar()`.

No caminho DAMP do carrier, `calcEnv()`:
- reconhece o limiar `(env >> 2) == 0x1f`;
- seleciona DECAY se AR=15, caso contrário ATTACK;
- zera os acumuladores PG do carrier e do modulator;
- sincroniza o estágio EG do modulator.

A sequência de software implica que o modulator já executou PG/EG e calculou seu output antes do carrier chegar à transição. O carrier também calcula PG antes de seu EG executar o reset dos dois acumuladores. A saída carrier subsequente usa o nível EG já atualizado. O próximo cálculo de fase encontrará os acumuladores zerados.

Essa ordem de chamadas é mais específica que a mera existência do evento DAMP→ATTACK/DECAY. Ainda assim, não identifica o instante interno de slot/subfase em que o silício realiza cada operação.

## 3. Confronto com Rust atual

| Aspecto | Modelo Andete 2017 | Rust atual | Resultado |
|---|---|---|---|
| Dono do contador global | `YM2413::counter` | chamador fornece `global_counter` | contrato externo preservado; falta um owner temporal integrado |
| Incremento | uma vez no início de `calc()` | não ocorre no EG nem no banco | Rust não presume pre/post-incremento |
| Valor compartilhado | mesmo valor para todos os operadores daquela chamada | cada tick recebe um argumento | chamador pode fornecer valor comum, mas a API não o impõe |
| Ordem por operador | PG antes de EG | PG e EG são chamadas separadas | sequência ainda não é codificada |
| Ordem do canal | modulator antes do carrier | APIs permitem ambas as ordens | scheduler ainda não define ordem |
| DAMP carrier | sincroniza modulator e zera os dois PGs | evento tipado e consumo atômico configurável | semântica candidata exposta, fronteira temporal externa |
| Saída | usa PG calculado antes do EG e nível EG após update | avaliação do par é chamada separadamente | pipeline de amostra ainda não integrado |

A implementação Rust em `EnvelopeGenerator::tick_with_events()` já reflete uma parte importante da ordem do modelo: ao entrar em uma atualização com DAMP concluído, seleciona ATTACK/DECAY antes de avaliar o estágio novo, permitindo que o novo estágio consuma o mesmo `global_counter`. A remoção do retorno antecipado foi integrada anteriormente na PR #1050.

O banco em `synthesis.rs` mantém separadas as chamadas `advance_operator_phase()`, `tick_operator_envelope_with_events()`, avaliação do par e consumo do evento de reset. Isso é intencional enquanto o instante físico não estiver demonstrado.

## 4. Decisões

1. **Não alterar produção nesta auditoria.** A ordem C++ documenta uma referência útil, mas ainda não é prova física suficiente para fechar o sequenciador.
2. Preservar o contador EG como entrada explícita. Não usar `completed_rotations` automaticamente e não impor fase zero.
3. O futuro owner temporal deve produzir um único identificador/valor de amostra global por rotação e fornecê-lo de modo consistente aos 18 EGs que pertençam à mesma atualização lógica, se a integração experimental confirmar essa unidade.
4. A ordem modulator→carrier e PG→EG deve ser mantida como **hipótese de modelo** em vetores diferenciais, sem codificá-la como contrato cycle-accurate antes de captura física.
5. O evento DAMP/reset PG deve ser testado em três momentos distintos: antes do cálculo PG, entre PG e avaliação EG, e após a saída do operador. A comparação física precisa determinar qual momento reproduz o chip.
6. Não inferir que os quatro clocks de um slot correspondam diretamente a essas chamadas C++; o modelo é amostrado por saída e não fornece offsets internos por subfase.

## 5. Próximos testes determinísticos permitidos

Antes de escrever um sequenciador:
- construir vetores de referência para um único canal com modulator/carrier e contador global fixo por amostra;
- cobrir o carrier cruzando DAMP no mesmo update em que ATTACK/DECAY é selecionado;
- verificar a fase PG observável antes/depois do evento, sem assumir qual valor deve ser exposto como saída física;
- verificar que o valor do contador não muda entre os dois operadores nem entre canais dentro da mesma chamada de amostra;
- manter explícitos os estados iniciais de PG, EG e feedback.

Esses testes caracterizam equivalência com o modelo Andete, não equivalência com hardware. A validação física continua dependente de capturas sincronizadas de XIN e MO/RO e, idealmente, do modo de diagnóstico interno documentado na investigação P1.

## 6. Conclusão

A auditoria resolve a ambiguidade **do modelo Andete**: contador incrementado no início da amostra; valor compartilhado durante todos os operadores; modulator antes do carrier; PG antes de EG em cada operador; DAMP do carrier reseta os dois PGs depois de o PG do carrier ter sido calculado naquela chamada.

Ela não resolve a fase absoluta do contador físico, o instante real de incremento, os offsets dos estágios nos quatro clocks de slot nem o ponto de consumo do reset PG no YM2413. FM-1/FM-3 permanecem parciais e FM-6 continua pendente.


## 4.1 Anomalia na expressão de RATE da referência C++

A leitura literal de `model5.cc` também encontrou:

```cpp
uint6 rate = max(63, 4 * basicRate + rks);
```

No mesmo arquivo, `uint6` é declarado como alias de `uint8_t`, e não como um tipo C++ com truncamento automático para seis bits. Com `std::max`, a expressão seleciona o maior valor entre 63 e a taxa calculada; ela não implementa saturação superior em 63. Isso é inconsistente com a descrição de RATE como campo de seis bits e com as faixas de taxa usadas nas próprias tabelas/medições do modelo.

**Tratamento:** não copiar essa expressão para Rust nem inferir silenciosamente que o autor pretendia `min`. O Rust atual limita a taxa a 63 em `EnvelopeParameters::effective_rate()`; essa decisão deve continuar rastreável à especificação do campo de seis bits e às medições, não ser justificada pela expressão C++ literal. A anomalia precisa permanecer registrada como possível erro tipográfico/defeito do modelo de referência até confirmação pelo autor ou por fonte primária adicional.
