# Road Rash — travamento da animação do letreiro na tela de título

- **Data do registro:** 2026-11-15
- **Sistema:** Sega Master System (SMS)
- **Jogo:** Road Rash
- **Classificação:** observação empírica / investigação pendente
- **Estado:** não corrigido; nenhuma alteração de hardware ou temporização proposta nesta etapa

## 1. Sintoma observado

Durante execução manual do jogo no Sega Emulator, foi observado o seguinte comportamento na tela de título:

1. O jogo inicializa e alcança a tela de título.
2. O nome/logotipo do jogo, que deveria entrar da direita para a esquerda com uma ondulação formada por sprites, não aparece em movimento.
3. Sem interação, o letreiro permanece parado.
4. Ao pressionar repetidamente o botão **Pause** do Master System, o letreiro avança em pequenos passos, aproximadamente um frame por pressão, com movimento visual irregular.
5. Depois que o letreiro termina de se posicionar e a ondulação termina, o jogo prossegue normalmente e fica jogável. Foi possível completar duas corridas durante o teste.

Ainda não foi determinado se, enquanto o letreiro está parado, o restante da tela permanece animado ou se toda a imagem fica congelada. Essa distinção deverá ser registrada no próximo teste.

## 2. O que esta observação permite concluir

- O jogo consegue inicializar e alcançar a tela de título.
- A sequência de animação do letreiro não progride espontaneamente no cenário observado.
- A intervenção repetida com Pause altera o progresso observável da sequência.
- Após essa sequência, o jogo consegue entrar em gameplay funcional no teste manual realizado.

Isso **não** demonstra que o Z80 esteja semanticamente incorreto, que esteja permanentemente em HALT, nem que exista uma falha específica no VDP. Também não prova que a causa esteja restrita ao VDP ou ao jogo.

## 3. Hipóteses a investigar — não confirmadas

O botão Pause do Master System está associado à entrada NMI do Z80. Portanto, a recuperação parcial após pressioná-lo repetidamente justifica investigar a interação entre interrupções, execução da CPU e eventos temporais do VDP/Scheduler.

Hipóteses abertas:

1. **Estado de execução/espera do Z80:** a CPU pode estar em HALT ou em uma rotina de espera cuja progressão dependa de uma interrupção/evento que não esteja sendo observado como esperado.
2. **Fase de interrupção VDP–CPU:** VBlank/VINT, HINT ou a entrega/observabilidade da linha INT podem estar desalinhados em relação à amostragem da CPU.
3. **Progressão da rotina de animação:** a rotina que atualiza a posição/animação dos sprites do letreiro pode não estar sendo executada ou retomada no ritmo esperado.
4. **Interleaving temporal:** a interação entre execução interruptível da CPU, barramentos, VDP e MachineScheduler pode expor uma divergência de fase que não aparece em testes isolados.

Essas hipóteses não são mutuamente exclusivas. O fato de o jogo ficar jogável depois da animação sugere que o sintoma pode estar concentrado nessa fase, mas não identifica sua causa.

## 4. Relação com a investigação de timing do VDP

O caso deve ser tratado como um teste de integração real **Z80–VDP–barramentos–MachineScheduler**, e não como uma exceção específica de Road Rash.

A investigação MAME sobre regressões do Road Rash após a adoção de execução Z80 interruptível é contexto comparativo: ela demonstra que mudanças na granularidade/interleaving da CPU podem expor diferenças temporais na integração CPU–VDP. Não prova que o mesmo defeito exista neste projeto, nem autoriza copiar constantes HPOS de MAME diretamente para o Scheduler.

As lacunas gerais de VDP (Frame/Line IRQ HCount, VCounter change time, IRQ VCount, colisão/overflow HCount e fetch) continuam sendo investigadas separadamente. Este registro não altera seu escopo nem sua ordem.

## 5. Plano de diagnóstico para retomada

Quando esta investigação for retomada, capturar a primeira divergência reproduzível sem introduzir correções antecipadas.

Comparar três execuções equivalentes:

- A: deixar a tela de título parada sem pressionar Pause;
- B: pressionar Pause uma única vez;
- C: pressionar Pause repetidamente até a animação terminar.

Usar trace/telemetria existente e, quando disponível, correlacionar no mesmo domínio temporal:

- timestamp absoluto / master clock;
- PC e estado HALT do Z80;
- IFF1/IFF2 e estado/pending de NMI e IRQ;
- estado de VBlank/VINT/HINT e linha INT do VDP;
- próximo evento e fase do MachineScheduler;
- eventos relevantes de leitura/escrita de barramento e atualização/publicação do frame.

Os nomes acima descrevem os sinais necessários; a instrumentação deverá reutilizar os estados e APIs reais do runtime, sem criar cópias de estado ou callbacks por instrução.

Também registrar se o fundo continua animando enquanto o letreiro está parado. Preservar ROM, perfil/revisão do VDP, região/clock, configuração e hash do build usados no teste, se esses dados estiverem disponíveis.

## 6. Critérios para uma futura correção

Nenhuma alteração em semântica Z80, boundaries de VDP, clocks ou MachineScheduler deve ser feita apenas para fazer Road Rash avançar.

Uma correção futura exige:

1. primeira divergência temporal reproduzível e evidência correlacionada CPU/VDP/Scheduler;
2. confronto com documentação SMS Power e testes diagnósticos aplicáveis;
3. comparação independente com Ares, MAME e/ou Gearsystem, sem tratar qualquer implementação como oráculo absoluto;
4. teste automatizado que reproduza o comportamento sem depender de pressionamentos manuais;
5. equivalência entre execução contínua e em blocos;
6. snapshot/restore antes e depois do evento, quando aplicável;
7. regressões SMS1, SMS2 e SG-1000 pertinentes;
8. validação Release e Windows x64 quando o escopo da alteração exigir.

## 7. Limite desta PR

Esta PR registra o sintoma e preserva o contexto para investigação posterior. Não implementa correção, não altera constantes temporais, não modifica Z80/VDP/Scheduler e não declara Road Rash compatível.

**Manter esta PR aberta** até que o diagnóstico seja retomado e a causa seja confirmada ou a observação seja refutada por reprodução controlada. O trabalho imediato poderá continuar nas lacunas gerais do VDP sem depender do fechamento deste caso.
