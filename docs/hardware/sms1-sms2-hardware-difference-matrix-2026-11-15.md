# SMS1 × SMS2 — matriz de diferenças de hardware

**Estado:** AUDIT BASELINE — não representa conclusão de implementação
**Data:** 2026-11-15
**Escopo:** Master System com ASIC VDP Sega 315-5124 (SMS1) e 315-5246 (SMS2)
**Princípio:** distinguir o modelo comercial da placa efetiva e da revisão do ASIC. A literatura registra exceções: 315-5124 e 315-5246 não são identificadores infalíveis do gabinete/modelo comercial.

## 1. Resultado executivo

A documentação pública de SMS Power identifica duas diferenças centrais entre os VDPs: o 315-5124 mantém efeitos de bits legados TMS9918 na geração de endereços VRAM; o 315-5246 remove esses masks e acrescenta os modos de 224 e 240 linhas. A documentação de sprites registra ainda que o zoom horizontal do 315-5124 não se aplica aos quatro últimos sprites selecionados da linha, enquanto o 315-5246 aplica zoom completo aos oito. A documentação de paleta registra o primeiro nível de azul ligeiramente elevado no 315-5124, ausente no 315-5246.

Não há evidência suficiente para afirmar uma substituição geral de CPU, RAM, VRAM, mapper ou PSG entre os modelos. Esses blocos devem permanecer compartilhados no runtime até que uma revisão de placa ou medição demonstre uma diferença.

## 2. Matriz comparativa

| Subsistema | SMS1 / 315-5124 | SMS2 / 315-5246 | Estado da evidência | Consequência para o emulador |
|---|---|---|---|---|
| Identidade do VDP | ASIC 315-5124 | ASIC 315-5246 | Confirmado | Seleção explícita de revisão; não inferir pela região ou apenas pelo gabinete |
| CPU | Z80A Sega 315-0041 nas listas de peças consultadas | Z80A Sega 315-0041 nas listas de peças consultadas | Compartilhado nas placas consultadas; outras revisões devem ser catalogadas | Reutilizar Z80 e integração temporal |
| Clock CPU/PSG | Derivado do padrão regional e clock da placa | Derivado do padrão regional e clock da placa | Não foi demonstrada diferença intrínseca causada pelo ASIC | Região/clock permanece independente da revisão do VDP |
| VDP — modos | Mode 4 até 192 linhas; modos legados TMS9918 | Mode 4 com 192, 224 e 240 linhas; modos legados | Diferença confirmada | Perfil 315-5246 habilita alturas estendidas; SMS1 deve permanecer em 192 |
| VDP — endereçamento de tabelas | Bits de máscara legados em R2/R5/R6 podem forçar linhas do endereço VRAM a zero | Esses bits não mascaram os endereços | Diferença confirmada | Manter funções de geração de endereço condicionadas ao ASIC |
| VDP — sprites/zoom | Em zoom 2×, os últimos quatro sprites encontrados na linha não recebem zoom horizontal; zoom vertical continua | Os oito sprites podem receber zoom horizontal e vertical | Diferença confirmada | Avaliação/composição de sprites precisa considerar revisão; não duplicar SAT sem evidência |
| VDP — paleta/DAC | Primeiro nível de azul ligeiramente mais brilhante em 16 cores | Sem esse realce | Diferença confirmada | Conversão CRAM→RGB precisa ser perfilada pelo ASIC |
| VDP — SAT e limite por linha | Até 64 sprites; até 8 selecionados por linha | Até 64 sprites; até 8 selecionados por linha | Compartilhado na documentação geral; fase sublinha ainda exige validação | Capacidade comum; timing e latch devem ser testados por perfil |
| VDP — HINT/VINT/status | Mesmos registradores e interface de status documentados | Mesmos registradores e interface de status documentados | Interface compartilhada; diferenças exatas de fase não comprovadas aqui | Não copiar timing entre perfis sem testes de hardware |
| VDP — portas | Dados $BE e controle/status $BF | Dados $BE e controle/status $BF | Compartilhado | Barramento/decodificador comum com estado interno explícito |
| Controle VDP — primeiro byte | O primeiro byte pode atualizar somente a parte baixa do endereço; a ação de modo/read-ahead depende do segundo byte | Mesmo contrato geral descrito para a porta de controle | Evidência documental geral; diferenças de microtiming entre ASICs não estabelecidas | Testar latch parcial e read-ahead por revisão; não supor equivalência subciclo |
| PSG digital | SN76489 integrado ao VDP | SN76489 integrado ao VDP | Compartilhado; SMS Power registra o mesmo padrão de ruído amostrado para SMS1/SMS2 | Reutilizar o core PSG e a sincronização do Scheduler |
| PSG — canais/registradores | 3 tons + 1 ruído; 4 atenuadores | 3 tons + 1 ruído; 4 atenuadores | Compartilhado | Nenhum PSG SMS2 separado justificado |
| RAM principal | 8 KiB nas placas consultadas | 8 KiB nas placas consultadas | Capacidade compartilhada; chips/revisões variam | Reutilizar memória base; perfil físico de placa permanece extensível |
| VRAM | 2 × 8 KiB = 16 KiB nas placas consultadas | 2 × 8 KiB = 16 KiB nas placas consultadas | Capacidade compartilhada | Reutilizar armazenamento de 16 KiB; decodificação pertence ao VDP |
| CRAM | Interna ao VDP; 32 entradas de 6 bits no modo SMS | Interna ao VDP; 32 entradas de 6 bits no modo SMS | Compartilhado quanto à organização; saída RGB difere pela característica azul | Não modelar CRAM como RAM externa; saída RGB por revisão |
| Mapper/cartucho | Mappers do ecossistema SMS | Mappers do ecossistema SMS | Nenhuma diferença de mapper por modelo demonstrada | Mapper pertence ao cartucho; manter independente do modelo de console |
| Memory map / barramento CPU | Mapa SMS e portas padrão | Mapa SMS e portas padrão | Compartilhado no contrato público; detalhes de board decoder podem variar | Um barramento compartilhado com perfis de I/O somente quando evidenciados |
| I/O controller / gate array | 315-5216 ou 315-5204 em placas consultadas | 315-5237 em placas PAL consultadas; 315-5216 em placa RGB | Dependente da revisão/placa, não uma diferença universal SMS1×SMS2 | Modelar revisão de placa separadamente; não atribuir toda diferença ao VDP |
| Encoder de vídeo | Varia por região e placa (ex.: V7040/MC1377) | Varia por região e placa (ex.: CXA1145P ou ausência em RGB) | Dependente da placa/região | Fora do framebuffer digital; futura modelagem analógica separada |
| Áudio analógico | Circuito de saída e componentes variam por placa | Circuito de saída e componentes variam por placa | Não caracterizado comparativamente | Não inventar filtro/ganho específico de SMS2 |
| Interrupções | HINT/VINT disponíveis | HINT/VINT disponíveis | Sem diferença geral comprovada; fase exata por ASIC ainda aberta | Medir fronteiras, status e IRQ em cada perfil |
| Reset/BIOS | BIOS e circuito de reset variam por região/placa | BIOS e circuito de reset variam por região/placa | Dependente da placa | Separar ROM BIOS e reset elétrico do modelo de VDP |

## 3. Implicações arquiteturais

### 3.1 Identidade do hardware

A configuração executável deve representar, no mínimo, dimensões independentes:

- **Console/modelo selecionado:** Master System (perfil SMS1) ou Master System II (perfil SMS2).
- **ASIC VDP efetivo:** 315-5124 ou 315-5246.
- **Padrão de vídeo:** NTSC, PAL-M ou PAL.
- **Revisão de placa/I/O:** desconhecida até ser identificada; não derivar automaticamente apenas do modelo comercial.

A seleção de SMS2 no menu deve escolher explicitamente o perfil 315-5246. A seleção de região altera clock e temporização regional, não troca o ASIC. Para placas excepcionais ou modificadas, o ASIC efetivo deve poder ser sobrescrito por uma opção avançada ou perfil de placa, com a identidade exibida no HUD/log.

### 3.2 Reutilização

Devem permanecer compartilhados, salvo nova evidência:

- Z80 e seu sequenciador temporal;
- Scheduler e contrato de barramento;
- 8 KiB de RAM principal e 16 KiB de VRAM como capacidades de referência;
- core SN76489 digital;
- lógica comum de portas e mappers de cartucho.

O VDP deve permanecer uma implementação com revisão explícita, não duas cópias divergentes. Condicionais por ASIC são aceitáveis quando descrevem diferenças comprovadas e possuem testes contratuais.

### 3.3 Diferenças VDP já identificadas no código

O código em `native/video_core/src/sms/vdp.rs` já possui condicionais por `SmsVdpRevision` para:
- altura ativa 192/224/240;
- geração de endereços de SAT e tabelas;
- política de zoom de sprites;
- paleta RGB;
- alguns caminhos de status/IRQ e composição.

A existência dessas condicionais não comprova que a temporização SMS2 esteja completa. Em especial, o buffer persistente de avaliação da próxima linha foi introduzido para SMS1 e a documentação atual do projeto declara que SMS2 ainda utiliza o caminho legado nessa etapa.

## 4. Lacunas que bloqueiam declarar SMS2 completo

1. Seleção explícita persistida no desktop e propagada até `SmsTemporalWorker`.
2. Configuração do 315-5246 independente de região, ROM e mapper.
3. Testes contratuais específicos de máscara de endereços R2/R5/R6.
4. Testes de zoom para 1–8 sprites e evidência da ordem de avaliação.
5. Testes de paleta que comprovem o realce de azul apenas no 315-5124.
6. Testes de 224/240 linhas, contadores, HINT/VINT e interrupções no perfil 315-5246.
7. Auditoria do caminho SMS2 de sprite evaluation/latch; não presumir que o buffer SMS1 representa o ASIC SMS2.
8. Validação de framebuffer dinâmico no Windows com 192/224/240 linhas.
9. Validação funcional de Cosmic Spacehead (CRC32 29822980) do menu ao gameplay.
10. Regressões SMS1 e SG-1000.

## 5. Evidências e referências

### Documentação primária/comunitária técnica

- SMS Power — [SMS1/SMS2 VDPs](https://www.smspower.org/Development/SMS1SMS2VDPs): identifica 315-5124/315-5246, masks de endereço legado e modos estendidos.
- SMS Power — [VDP Registers](https://www.smspower.org/Development/VDPRegisters): semântica dos masks em R2/R5/R6 e bits de modo.
- SMS Power — [Sprites](https://www.smspower.org/Development/Sprites): limite, ordem SAT e diferença de zoom entre SMS1 e SMS2.
- SMS Power — [Palette](https://www.smspower.org/Development/Palette): realce do primeiro nível de azul no SMS1.
- SMS Power — [SN76489](https://www.smspower.org/Development/SN76489): comportamento do PSG e medições de ruído SMS1/SMS2.
- SMS Power — [SMS Master System Service Manual](https://www.smspower.org/Development/SegaMasterSystemServiceManual): lista de peças das placas Master System consultadas.
- SMS Power — [SMS II Service Manual](https://www.smspower.org/Development/SegaMasterSystemIIServiceManual): lista de peças das placas Master System II consultadas.
- SMS Power — [I/O Port Map](https://www.smspower.org/Development/IOPortMap): portas e espelhamentos documentados.

### Regra de evidência

A documentação acima estabelece diferenças conhecidas, mas não fornece uma caracterização elétrica completa de todas as revisões. Questões não documentadas devem permanecer marcadas como desconhecidas até teste independente, captura de barramento ou análise de placa. Ares e Gearsystem são comparadores, não oráculos normativos.
