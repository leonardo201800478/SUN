# Temporal Core — clocks e contratos do Z80/SMS

**Status:** pesquisa preliminar. Os números abaixo são candidatos e evidências, não uma decisão final de unidade temporal.

## 1. Evidência do hardware SMS

O manual de serviço da Sega registra:
- sinal de 10.738635 MHz no pino 30 do VDP;
- clock de 3.579545 MHz no pino 34 do VDP, ligado ao pino de clock do Z80.

Fonte primária: [Sega Master System Service Manual (PDF)](https://segaretro.org/images/7/7b/SMSServiceManualEU.pdf), página impressa 4.

A página Clock Rate do SMS Power descreve o cristal mestre NTSC em torno de 53.6931 MHz e sua divisão por 15 para obter o clock de sistema em torno de 3.57954 MHz. Para PAL, indica aproximadamente 53.2034 MHz dividido por 15, resultando em aproximadamente 3.546893 MHz.

Fonte: [SMS Power — Clock Rate](https://www.smspower.org/Development/ClockRate).

O código SMS do ares usa 684 clocks horizontais por scanline e incrementa o contador horizontal em cada clock VDP. A documentação técnica do VDP descreve H/V counters, leitura do V counter, latch do H counter via TH e 262 linhas NTSC / 313 PAL.

Fontes:
- [ares SMS VDP](https://github.com/ares-emulator/ares/blob/master/ares/ms/vdp/vdp.cpp)
- [Sega Master System VDP documentation — Charles MacDonald](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt).

## 2. Candidato de base comum de clock NTSC

A combinação das frequências de serviço sugere uma possível base inteira comum:

- 10.738635 MHz × 5 = 53.693175 MHz.
- 3.579545 MHz × 15 = 53.693175 MHz.
- Se a linha tiver 684 clocks VDP, então 684 × 5 = 3420 ticks da base comum.
- Se o Z80 executar 228 T-states por linha, então 228 × 15 = 3420 ticks da mesma base.

O número 3420 também aparece como M-cycles por linha nos comentários do Genesis Plus GX.

Fonte: [Genesis Plus GX — core/system.c](https://github.com/ekeeke/Genesis-Plus-GX/blob/master/core/system.c).

**Interpretação:** um clock mestre inteiro poderia representar, sem ponto flutuante e sem frações acumuladas, a relação entre VDP e Z80 no SMS NTSC. Isso merece protótipo matemático.

**Não aprovado ainda:** precisamos confirmar a definição exata de cada clock, a fase relativa dos sinais, os modelos/revisões de VDP e a relação real com PSG/YM2413. Também precisamos confirmar se a mesma representação cobre PAL e eventos sub-T-state que sejam relevantes. Não transformar uma coincidência de frequências em contrato de hardware sem validar todas as premissas.

## 3. Z80 — granularidade temporal obrigatória

O manual oficial Zilog UM008011-0816 define:
- instruções compostas por operações básicas e ciclos M;
- M1 (opcode fetch) com quatro, cinco ou seis T-states conforme operação e WAIT;
- operações de memória normalmente com três T-states, extensíveis por WAIT;
- WAIT amostrado na borda de descida do clock durante T2 e em cada estado TW subsequente;
- leitura de memória amostrada em T3;
- ciclo de I/O com um estado WAIT inserido automaticamente;
- INT amostrado na borda de subida do último clock ao final da instrução;
- NMI sensível à borda negativa e reconhecida ao fim da instrução;
- ciclo de acknowledge de INT com dois estados WAIT automáticos.

Fonte primária: [Zilog Z80 CPU User Manual UM008011-0816 (PDF)](https://www.homecomputermuseum.nl/wp-content/uploads/2025/02/Z80_CPU_Users_Manual_2016.pdf), especialmente páginas impressas 7–13.

### Consequências para o contrato temporal

O Temporal Core não pode assumir que uma instrução inteira é uma unidade atômica suficiente para todos os casos. A integração futura com o Z80 precisa conseguir:
- avançar até fases observáveis de ciclos M/T;
- parar antes de ultrapassar uma fronteira relevante;
- amostrar WAIT no instante correto;
- suspender e retomar uma transação sem repetir efeitos;
- aplicar leituras/escritas na fase correta do ciclo;
- amostrar INT/NMI nos limites definidos pelo processador;
- representar ciclos estendidos e acknowledge de interrupção.

O Temporal Core não implementará a lógica interna do Z80. Ele fornecerá o mecanismo temporal e os contratos necessários para que o Z80 e o barramento cooperem corretamente.

## 4. Casos de teste derivados das fontes

Antes do código de produção, os modelos de referência devem cobrir:

1. Relação exata entre ticks mestre, clocks VDP e T-states Z80.
2. Acúmulo de fase para razões inteiras e não inteiras.
3. NTSC/PAL sem deriva ao longo de sessões extensas.
4. Acesso de memória com amostragem de dados em T3.
5. WAIT solicitado antes de T2, amostrado em T2 e mantido por múltiplos TW.
6. I/O com WAIT automático e WAIT adicional.
7. INT assertado antes, durante e depois do último T-state da instrução.
8. NMI por borda, incluindo pulso que não coincide com início de instrução.
9. Eventos VDP que coincidem com acesso CPU/barramento.
10. Snapshot/restore no meio de um ciclo de barramento com WAIT pendente.

## 5. Decisões em aberto

- Base mestre de 53.693175 MHz (NTSC) e 53.203424 MHz (PAL), ou outra unidade inteira derivada?
- O VDP será expresso em clocks de chip, dots ou ticks mestres?
- O CPU será expresso em T-states e fases de M-cycle, com conversão exata?
- Qual fase relativa deve ser preservada entre CPU, VDP, PSG e YM2413?
- Os clocks dos modelos e revisões de hardware variam?
- A API temporal deverá permitir sub-T-state para casos que o hardware exija, ou T-state + fase de barramento será suficiente?

Nenhuma destas decisões está fechada nesta pesquisa preliminar.
