# Temporal Core — divergências e qualidade das fontes

**Status:** registro de conflitos encontrados durante a pesquisa. Nenhuma divergência deve ser resolvida por suposição.

## 1. PSG: mapa de portas I/O

O guia de plataformas do 8bitworkshop lista `$F0-$F2` como portas de som/PSG.

Já a documentação técnica do VDP de Charles MacDonald e a documentação oficial do SMS Power descrevem:
- `$7E`: V counter na leitura / PSG na escrita;
- `$7F`: H counter na leitura / PSG na escrita (espelho);
- `$BE`: VDP data;
- `$BF`: VDP control.

Fontes:
- [8bitworkshop — Sega Master System](https://8bitworkshop.com/docs/platforms/sms/)
- [SMS Power — SMS Official Docs](https://www.smspower.org/Development/SMSOfficialDocs)
- [Charles MacDonald — Sega Master System VDP documentation](https://github.com/franckverrot/EmulationResources/blob/master/consoles/sms-gg/Sega%20Master%20System%20VDP%20documentation.txt)

**Tratamento:** não usar o quadro de portas do 8bitworkshop como especificação de hardware. Priorizar documentação primária/mais específica e confirmar por testes de portas e ROMs de diagnóstico. O guia 8bitworkshop continua útil para visão geral de desenvolvimento, não para definir sozinho o contrato temporal.

## 2. Frequência nominal do clock NTSC

O manual de serviço Sega informa 10.738635 MHz no pino de clock do VDP e 3.579545 MHz no clock entregue ao Z80, ambos com tolerâncias de medição declaradas.

A página Clock Rate do SMS Power descreve o cristal mestre como aproximadamente 53.6931 MHz e a divisão por 15 como aproximadamente 3.579540 MHz. A diferença aparente decorre de valores arredondados/truncados e da distinção entre frequência nominal e frequência de subportadora de cor.

Fontes:
- [Sega Master System Service Manual](https://segaretro.org/images/7/7b/SMSServiceManualEU.pdf)
- [SMS Power — Clock Rate](https://www.smspower.org/Development/ClockRate)

**Tratamento:** registrar os valores nominais, sua fonte, tolerância e derivação. Não misturar frequência real do oscilador, frequência de subportadora de vídeo e valor arredondado em metadados de ROM/VGM. O perfil de clock do V2 deverá declarar explicitamente qual valor usa e por quê.

## 3. Limite de precisão da documentação VDP

A documentação de Charles MacDonald descreve counters, linhas, flags e condições de IRQ, mas registra explicitamente que não dispunha de dados suficientes sobre o instante exato dentro da scanline para alguns eventos.

**Tratamento:** usar o documento como base de comportamento e não como prova de microtiming completo. Complementar com código de emuladores, testes SMS VDP e observação de hardware quando disponível.

## 4. Regras de precedência das fontes

1. Datasheet/manual do fabricante e manual de serviço Sega.
2. Documentação técnica específica do hardware com autor e limitações identificados.
3. Testes de hardware/ROMs de diagnóstico com resultado reproduzível.
4. Código de emuladores conceituados, estudado na versão/commit correspondente.
5. Guias de desenvolvimento e resumos gerais.
6. Hipóteses derivadas do modelo matemático.

Código de emulador é evidência de uma implementação, não prova automática de comportamento físico. Quando fontes divergem, registrar o conflito e criar um teste que possa discriminá-las.
