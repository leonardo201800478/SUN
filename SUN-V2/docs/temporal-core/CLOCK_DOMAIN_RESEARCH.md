# Temporal Core — clocks e contratos do Z80/SMS

**Status:** pesquisa R0 em andamento. Valores nominais, valores derivados e escolhas de modelagem estão separados. Nenhuma unidade temporal ou fase absoluta foi aprovada.

## 1. Nomenclatura: não existe um único número chamado “master clock”

As fontes usam nomes diferentes para sinais em pontos distintos da cadeia:

- cristal/oscilador de referência da placa;
- entrada de clock do VDP;
- clock do Z80, também usado como referência de entrada do PSG;
- clock de pixel/dot usado pelo modelo de vídeo;
- clocks internos divididos do PSG e YM2413;
- tempo lógico escolhido pelo emulador.

O V2 não deve chamar todos esses sinais de `master_clock`. Cada perfil precisa declarar o pino/sinal, frequência, derivação, tolerância, revisão da placa e fase conhecida.

## 2. NTSC — evidência forte

O manual de serviço Sega registra no circuito IC5/VDP:

- pino 30: 10.738635 MHz ±100 Hz;
- pino 34, ligado ao clock do Z80: 3.579545 MHz ±30 Hz.

Fonte primária: [Sega Master System Service Manual (PDF)](https://segaretro.org/images/7/7b/SMSServiceManualEU.pdf), página impressa 4 / linhas 41–45 da extração.

A configuração SMS1 NTSC do MAME usa:
- VDP: 10.738635 MHz;
- Z80: 10.738635 / 3 = 3.579545 MHz;
- YM2413 no perfil japonês: 10.738635 / 3.

Fontes de implementação: [MAME sms.cpp](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp), funções `sms1_ntsc` e `smsj`.

A página SMS Power descreve o oscilador NTSC como aproximadamente 53.6931 MHz e o clock de sistema como 3.579540 MHz por divisão por 15, observando que 3.579545 MHz é o valor oficial da subportadora 315/88 MHz. A derivação coerente com o manual Sega é:

`10.738635 MHz × 5 = 53.693175 MHz`  
`3.579545 MHz × 15 = 53.693175 MHz`

Fonte: [SMS Power — Clock Rate](https://www.smspower.org/Development/ClockRate).

**Conclusão limitada:** para o perfil NTSC documentado, 10.738635 MHz no VDP e 3.579545 MHz no Z80 são suportados por manual de serviço. A igualdade de frequência nominal do PSG com o Z80 é descrita pelo SMS Power; a fase elétrica exata ainda não está estabelecida.

## 3. PAL europeu — divergência de arredondamento

O código MAME define `MASTER_CLOCK_PAL = 53.203425 MHz`, explicitamente como 12 × 4.43361875 MHz. A configuração PAL usa:

- VDP: 53.203425 / 5 = 10.640685 MHz;
- Z80: 53.203425 / 15 = 3.546895 MHz;
- pixel clock da tela: 53.203425 / 10 = 5.3203425 MHz.

Fonte: [MAME sms.cpp](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp), `MASTER_CLOCK_PAL`, `sms_pal_base`, `sms1_pal`.

Há uma divergência de aproximadamente 2 Hz entre fontes:
- SMS Power apresenta 3.546893 MHz como clock PAL/SECAM nominal e deriva 3.546895 MHz a partir da subportadora;
- a referência de serviço PAL reproduzida em fontes técnicas cita aproximadamente 10.640684 MHz no VDP e 3.546893 MHz no Z80;
- MAME usa 53.203425 / 15 = 3.546895 MHz.

Fontes: [SMS Power — Clock Rate](https://www.smspower.org/Development/ClockRate); [Sega Master System Troubleshooting Guide — clock checks e referências ao manual](https://wiki.retrotechcollection.com/Sega_Master_System_Troubleshooting_Guide).

**Tratamento:** manter 3.546893 e 3.546895 como candidatos distintos. O perfil V2 não deve escolher um deles sem fixar a convenção (frequência medida do oscilador, valor nominal de subportadora ou derivação matemática) e tolerância.

## 4. PAL-M brasileiro — conflito material com o runtime V1

O perfil `brazil` do runtime V1 seleciona `VideoStandard::PalM`, mas fornece 3.579545 Hz ao worker/PSG e à conversão de tempo.

O MAME atual define:
- `MASTER_CLOCK_PALM = 10.726833 MHz`;
- comentário de fonte: valor nominal do XTAL PAL-M é 10.726834 MHz;
- valor 10.726833 é usado no modelo porque o valor nominal provoca erros no Flubba VDPTest, provavelmente por arredondamento do core;
- Z80 = `MASTER_CLOCK_PALM / 3` = 3.575611 MHz;
- VDP = `MASTER_CLOCK_PALM`;
- pixel clock = `MASTER_CLOCK_PALM / 2`;
- PAL-M usa contagem de linhas e parâmetros de tela NTSC.

Fonte: [MAME sms.cpp](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp), definições `MASTER_CLOCK_PALM`, `sms_br_base`, `sms1_br` e `sms3_br`.

**Conflito:** o valor de CPU/PSG configurado no runtime V1 (3.579545 MHz) difere do valor modelado pelo MAME para PAL-M (3.575611 MHz), diferença de 3.934 kHz. A diferença afeta tempo virtual, áudio e cadência de vídeo se o perfil for realmente PAL-M.

**Confiança:** alta para o que cada software configura; média para o valor físico PAL-M, pois o comentário do MAME descreve um ajuste do modelo e não foi localizada nesta rodada uma medição primária de placa TecToy.

SMS Power identifica a placa TecToy Power Base/PAL-M 199-403-411-416, com CPU Zilog Z0840004PSC e VDP Sega 315-5124, mas a página consultada não publica a frequência do oscilador. Fonte: [SMS Power — Brazilian Master System: TECTOY POWER BASE/PAL-M](https://www.smspower.org/Development/BrazilianMasterSystemTECTOYPOWERBASEPAL-M). A subportadora PAL-M brasileira é 3.575611 MHz, mas isso não prova, isoladamente, o clock CPU/PSG da placa. Fonte de padrão: [EPUSP — PAL-M](https://www.lcs.poli.usp.br/~gstolfi/PPT/APTV0416.pdf).

**Ação bloqueante:** obter manual/esquema de serviço PAL-M da TecToy ou medir a placa. Não “corrigir” o V1 neste trabalho documental nem promover o valor MAME a verdade física sem essa confirmação.

## 5. Relação VDP/Z80 e geometria horizontal

O MAME configura o clock de entrada do VDP em três vezes o clock do Z80 nos perfis NTSC, PAL e PAL-M. A tela usa metade do clock de entrada do VDP como pixel clock. No modelo NTSC isso resulta em:

- VDP: 10.738635 MHz;
- pixel clock: 5.3693175 MHz;
- Z80: 3.579545 MHz;
- razão VDP:Z80 = 3:1;
- razão pixel:Z80 = 3:2.

A documentação VDP de Charles MacDonald descreve 342 pixels por linha e 256 pixels ativos, mas adverte que parte dos dados foi tomada do manual TMS9918 e de testes em Genesis NTSC, e que um SMS real pode divergir.

Fonte: [Sega Master System VDP documentation — EmulationResources](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt), seção H counter. O MAME também mantém a implementação do VDP 315-5124/315-5246 em [315_5124.cpp](https://github.com/mamedev/mame/blob/master/src/devices/video/315_5124.cpp).

A relação matemática candidata para NTSC é:
- 342 pixels × 2 clocks VDP por pixel = 684 clocks VDP;
- 228 T-states Z80 × 3 clocks VDP por T-state = 684 clocks VDP.

Isso é uma razão consistente entre os valores do modelo, não prova de fase absoluta, instante de fetch ou equivalência para todas as revisões ASIC.

## 6. PSG e YM2413 — relação com a CPU

SMS Power descreve o PSG integrado como recebendo clock de entrada de 3.579545 MHz em NTSC e 3.546893 MHz em PAL/SECAM, com divisor interno /16. Para PAL-M, o valor deve acompanhar a resolução da seção anterior, não ser inferido por equivalência com NTSC.

No perfil Japanese SMS, MAME instancia YM2413 com `XTAL(10'738'635)/3`, isto é, 3.579545 MHz.

Fontes:
- [SMS Power — SN76489](https://www.smspower.org/Development/SN76489)
- [MAME sms.cpp — perfil SMS japonês](https://github.com/mamedev/mame/blob/master/src/mame/sega/sms.cpp), `smsj`
- [SMS Power — YM2413 Application Manual](https://www.smspower.org/maxim/Documents/YM2413ApplicationManual)

A frequência nominal não fixa fase inicial, alinhamento após reset, borda de atualização ou comportamento entre revisões de placa. Esses pontos continuam UNKNOWN.

## 7. Candidato matemático de unidade inteira — não aprovado

Para NTSC, a combinação do manual Sega sugere:

- 15 ticks candidatos por T-state Z80;
- 5 ticks candidatos por clock de entrada VDP;
- 10 ticks candidatos por pixel, se forem mantidos dois clocks VDP por pixel;
- 240 ticks candidatos por tick interno PSG (/16);
- 1080 ticks candidatos por período de amostra YM2413 (/72), caso o perfil use a mesma entrada de 3.579545 MHz.

A coincidência é útil para prototipagem matemática. Não aprova:
- unidade temporal pública;
- largura u64/u128;
- epoch;
- fase de reset;
- fase relativa entre CPU/VDP/PSG/FM;
- granularidade sub-T-state;
- fila ou estrutura de eventos.

PAL e PAL-M precisam ser avaliados com suas frequências e tolerâncias próprias. Um modelo que só fecha NTSC não fecha o requisito regional.

## 8. Z80 — granularidade temporal

O manual Zilog UM008011-0816 define:
- instruções compostas por operações básicas e ciclos M;
- M1 com quatro, cinco ou seis T-states conforme operação e WAIT;
- operações de memória normalmente com três T-states, extensíveis por WAIT;
- WAIT amostrado na borda de descida durante T2 e em cada TW subsequente;
- leitura de memória amostrada em T3;
- I/O com um estado WAIT automático;
- INT amostrado na borda de subida do último clock ao fim da instrução;
- NMI sensível à borda negativa e reconhecida ao fim da instrução;
- acknowledge INT com dois estados WAIT automáticos.

Fonte primária: [Zilog Z80 CPU User Manual UM008011-0816 (PDF)](https://www.homecomputermuseum.nl/wp-content/uploads/2025/02/Z80_CPU_Users_Manual_2016.pdf), páginas impressas 7–13.

O Temporal Core não implementará a lógica interna do Z80. Deve fornecer limites temporais e contratos para que CPU e barramento avancem, suspendam, retomem e apliquem efeitos exatamente uma vez.

## 9. Casos de teste derivados

1. NTSC: verificar relação 10.738635 MHz / 3.579545 MHz no perfil e contra fonte primária.
2. PAL: discriminar 3.546893 vs 3.546895 e registrar convenção e tolerância.
3. PAL-M: medir VDP input e clock Z80/PSG de uma placa TecToy identificada.
4. Medir fase relativa e reset phase CPU/VDP/PSG; YM2413 no perfil japonês.
5. Verificar 342 dots/linha, HCounter e limites de linha por ASIC/region.
6. Comparar clocks e contagem de linhas NTSC/PAL/PAL-M em sessão longa sem deriva.
7. Acesso de memória com amostragem T3 e WAIT amostrado em T2/TW.
8. I/O com WAIT automático e WAIT adicional.
9. INT/NMI nos limites de instrução e eventos VDP simultâneos.
10. Snapshot/restore no meio de request/WAIT e replay idêntico.

## 10. Decisões em aberto

- Qual convenção de frequência será armazenada: medição de oscilador, valor nominal de serviço ou derivação de subportadora?
- PAL-M TecToy usa 10.726833/3.575611 MHz na revisão-alvo ou há variantes?
- Qual fase relativa existe entre CPU, VDP e PSG?
- YM2413 recebe exatamente o clock CPU no Japanese SMS e qual é a fase após reset?
- 342 dots e 228 T-states por linha são universais para os ASICs SMS suportados?
- Qual granularidade abaixo de T-state é necessária para os consumidores de gameplay comprovados?

Nenhuma destas decisões está fechada nesta pesquisa.
