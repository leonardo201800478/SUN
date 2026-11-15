# Andre Agassi Tennis — análise estática da espera em C122

Data: 2026-11-14
Estado: **STATIC_ROM_ANALYSIS_CONFIRMED / RUNTIME_CAUSE_PENDING**
Runtime alterado: **não**
ROM/bytes comerciais no repositório: **não**

## Identidade local da ROM

A imagem fornecida para esta investigação foi analisada localmente. Somente metadados de identificação são registrados:

- Tamanho: 262.144 bytes (256 KiB)
- SHA-1: `81dbacad4739b98281c750d9af21606275398fc8`
- SHA-256: `522a2c8b34236f40a1f71afda104c1a27f11c52fdd1ffd7f63b8ef4121fe4224`
- CRC32: `F499034D`

Os hashes coincidem com o valor SHA-1/SHA-256 observado no diagnóstico local anterior. A ROM não foi copiada para o Git e nenhum byte, dump de memória ou sequência extensa de instruções foi incluído neste documento.

## Fluxo estático observado

Considerando o mapeamento direto da imagem sem cabeçalho, os endereços abaixo correspondem aos offsets da ROM:

- `$0088`: rotina que desabilita interrupções, lê a variável `$C122` e compara seu valor com `$15`.
- `$008E`: se a comparação for diferente, o desvio relativo conduz a `$00BD`.
- `$00BD`: reabilita interrupções e retorna por salto absoluto a `$0088`.
- Se `$C122 == $15`, o fluxo segue a partir de `$0090` em vez de retornar ao laço.

Isso explica estaticamente o padrão de PCs observado nas capturas: `$0088`, `$008C` e `$00BD`. O laço é uma espera condicionada por software; sua existência, isoladamente, não demonstra defeito no Z80.

## Escritas encontradas na imagem

Uma varredura por referências absolutas a `$C122` encontrou instruções de escrita nos seguintes offsets:

| Offset/endereço | Valor imediato observado | Interpretação limitada |
|---|---:|---|
| `$002B` | `$00` | Inicialização da variável |
| `$00E0` | `$03` | Escrita observada na rotina chamada durante o boot |
| `$0299` | `$12` | Escrita em outra rotina; alcançabilidade ainda não demonstrada |
| `$0CE8` | `$04` | Escrita em outra rotina; alcançabilidade ainda não demonstrada |

**Correção da hipótese anterior:** não é correto afirmar que a ROM só escreve `$03` em `$C122` ou que o valor nunca pode mudar. A imagem contém outras escritas candidatas. Ainda não foi demonstrado que qualquer uma delas seja executada no percurso reproduzido, nem que alguma deva produzir `$15`.

Há também outra rotina de leitura/comparação de `$C122` na região inicial da ROM. Sua relação com o laço principal e com o estado de interrupção deve ser reconstruída antes de atribuir semântica à variável.

## Correlação com o trace fornecido

O trace local analisado anteriormente continha:

- 2.048 eventos retidos e indicação de 23.331.332 eventos descartados antes da janela;
- 373 transações de barramento classificadas como leituras de memória;
- nenhuma escrita de memória ou I/O na janela retida;
- 55 leituras de `$C122`, retornando `$03`.

A janela é tardia e limitada. A ausência de escrita nela **não prova** que não houve escrita durante o boot ou fora da janela retida. O valor lido é compatível com a inicialização observada, mas não identifica quem deveria alterá-lo.

## Estado do debugger no repositório

As PRs #739, #741 e #743 foram integradas à `main`. A infraestrutura existente já oferece:

- snapshots coerentes e imutáveis do Z80/SMS pelo worker;
- trace temporal/barramento opt-in com histórico FIFO limitado a 2.048 eventos e contador de descarte;
- drenagem do trace no worker, sem transferir a máquina à GUI;
- comandos coarse-grained de pausa, execução e step de instrução;
- inspeção de memória pelo handle de diagnóstico, limitada a 256 bytes por solicitação;
- exportação local do trace sem bytes da ROM nem dump integral de memória.

A aplicação Slint expõe snapshot, ativação do modo Debugger e exportação do trace. A UI Slint atual não expõe diretamente todos os recursos da API de inspeção de memória/breakpoints; não presumir que o botão de snapshot execute uma captura de instruções.

## Próximo experimento obrigatório

O trace anterior não cobre o boot. A próxima captura deve começar imediatamente após um reset da máquina, com o modo Debugger ativado antes do reset:

1. Carregar a mesma ROM e registrar região/padrão de vídeo selecionado.
2. Ativar o modo Debugger (trace opt-in).
3. Reiniciar a máquina usando o controle Reset, mantendo o trace habilitado.
4. Deixar executar até o jogo entrar novamente no laço ou até o primeiro frame útil.
5. Exportar o trace imediatamente, antes que a janela FIFO descarte os eventos iniciais.
6. Conferir a ordem de leituras e escritas em `$C122`, escritas em `$C121`/`$C125`, I/O VDP/PSG, acknowledge de INT e transições de mapper.

O objetivo é localizar a primeira transição real da variável e identificar se a rotina que deveria alterar o estado é executada. Se o histórico de 2.048 eventos não preservar o boot completo, a próxima implementação deve adicionar uma captura de boot limitada por evento/tempo, com drenagem em blocos e checkpoints de CPU/mapper. Não aumentar o buffer sem limite.

## Hipóteses que permanecem abertas

1. A rotina que altera `$C122` para a fase seguinte não é alcançada.
2. A rotina é alcançada, mas uma condição de software/entrada impede a transição.
3. A transação de memória para a RAM em `$C122` é roteada ou observada incorretamente.
4. A interrupção/VDP ou o estado de entrada necessário à transição não é produzido/atendido.
5. O valor/fluxo esperado depende de região, BIOS/handoff ou mapper que ainda não foi confirmado.

Não forçar `$C122=$15`, não alterar Z80/VDP/PSG/Scheduler e não impor mapper/SP por suposição.

## Critérios para fechar a investigação

- [ ] Captura de boot desde reset, sem perda dos eventos iniciais.
- [ ] Fluxo de controle até cada escrita executável em `$C122` classificado.
- [ ] Primeira divergência entre ROM, CPU, RAM/barramento, IRQ/VDP e entrada identificada.
- [ ] Teste sintético determinístico que reproduza a falha isolada.
- [ ] Correção mínima com regressões SMS e SG-1000.
- [ ] CI PASS no SHA exato e reprodução manual no Windows.

Classificação: **STATIC_ROM_ANALYSIS_CONFIRMED**; **RUNTIME_CAUSE_PENDING**; **FIX_NOT_PROPOSED**; **TEMPORAL_PASS_NOT_CLAIMED**.
