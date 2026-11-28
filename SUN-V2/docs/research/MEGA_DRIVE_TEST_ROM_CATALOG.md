# Catálogo de fontes — test ROMs de Mega Drive

**Status:** referência catalogada para pesquisa e validação futura; nenhuma ROM foi baixada, executada ou aprovada como teste do SUN V2.

## Fonte principal

- **Título:** Development Tools/Software (Sega Mega Drive) — seção “Test ROMS”
- **URL:** https://techdocs.exodusemulator.com/Console/SegaMegaDrive/Software.html#test-roms
- **Escopo declarado pela fonte:** coleção não exaustiva de ROMs úteis para examinar o comportamento do hardware Mega Drive e verificar emuladores.
- **Classificação:** `FACT` quanto ao conteúdo publicado na página; a eficácia, integridade, licença e comportamento de cada ROM permanecem `UNKNOWN` até inspeção individual.
- **Uso no SUN:** referência para uma fase futura de Mega Drive/Genesis. Não integra o escopo atual SMS/CHRONOS, não altera gates R0–R5 e não autoriza implementação ou validação de Mega Drive antes da fase correspondente.

## ROMs catalogadas pela página

| Teste | Autor indicado | Finalidade descrita na fonte |
|---|---|---|
| IO Sample Program | Sega | Demonstra detecção de dispositivos de I/O e decodificação de entrada. |
| FM Test | DevSter | Teste básico de áudio FM via YM2612; sem saída visual. |
| Graphics & Joystick Sampler | Charles Doty | Gráficos simples e personagem móvel; útil como teste inicial de emulação. |
| 68000 BCD Verifier | flamewing | Verifica resultados de flags nas instruções BCD do Motorola 68000. |
| 68000 Opcode Sizes | r57shell | Verifica suporte aos tamanhos corretos dos opcodes 68000. |
| 68000 Illegal Opcodes | Charles MacDonald | Verifica exceções de instruções ilegais do 68000. |
| V counter test program | Charles MacDonald | Amostra valores observados do contador vertical do VDP em diferentes modos. |
| CRAM Flicker Test | Nemesis | Verifica posição e temporização de pontos CRAM na borda e na área ativa. |
| VDP Port Access Test ROM | Nemesis | Suite extensa para comportamento observável das portas do VDP, incluindo casos indefinidos ou incomuns. |
| VDP Sprite Masking Test ROM | Nemesis | Verifica mascaramento e overflow de sprites nos modos H32/H40 e casos-limite conhecidos. |
| VDP Test Register | Tristan Seifert | Teste interativo dos bits do registrador de teste do VDP, com observação gráfica e de áudio. |
| 68000 Memory Test | Charles MacDonald | Examina leituras de regiões indefinidas do mapa de memória 68000; resultados podem variar por modelo. |
| 1536 Color Test | Charles MacDonald | Demonstra cores via alterações dinâmicas de paleta e shadow/highlight. |
| Shadow-Highlight Test Program | Paul Lee | Demonstra o modo shadow/highlight. |
| Window Test | Fonzie | Demonstra Window, scroll dos planos A/B e o defeito de H-scroll descrito pela fonte. |
| Direct Color DMA | Chilly Willy | Testa o método Direct Color DMA; a fonte destaca a necessidade de temporização muito precisa. |
| Window distortion bug | Charles MacDonald | Demonstra o problema de H-scroll associado à Window alinhada à esquerda. |
| Overdrive | Titan | Demo técnica que explora limites, casos-limite e recursos não documentados. |
| Overdrive 2 | Titan | Continuação que exige recursos pouco comuns e casos-limite temporais. |

## Regras para incorporação futura ao harness

Antes de transformar qualquer item em teste automatizado do SUN:

1. Localizar e obter a ROM pela referência indicada, sem presumir que o link atual ou o binário permaneçam disponíveis.
2. Verificar licença, redistribuição permitida, autoria, versão e procedência.
3. Registrar nome, plataforma/revisão-alvo, região quando aplicável, tamanho e SHA-256 do arquivo exato.
4. Inspecionar o código-fonte quando disponível e documentar pré-condições, sinais de sucesso/falha e limites do teste.
5. Identificar quais resultados são determinísticos e quais dependem de modelo, revisão de placa, região ou temporização analógica.
6. Definir o método de execução e captura; não interpretar uma tela visual como prova de fidelidade completa.
7. Vincular cada execução ao SHA do SUN, ambiente, perfil de hardware emulado, configuração, resultado (`PASS`, `FAIL`, `BLOCKED`, `NOT RUN`) e artefatos.
8. Manter resultados de emuladores de referência como comparação, nunca como prova isolada do comportamento físico.

## Limites e separação de escopo

- Este catálogo é específico de **Mega Drive/Genesis**. Os testes 68000, YM2612, modos H32/H40, CRAM, DMA e demais comportamentos listados não são testes de SMS.
- A presença de uma ROM nesta lista não significa que tenha sido adquirida, auditada, executada, licenciada para redistribuição ou validada contra hardware real.
- A página declara que a lista não é exaustiva.
- A prioridade de desenvolvimento permanece SMS. O catálogo fica reservado para pesquisa e planejamento de uma fase futura de Mega Drive, sujeita a evidência técnica e autorização explícita no roadmap.
