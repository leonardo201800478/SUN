# SMS Firmware Manager — arquitetura de UI, preferências e resolução regional
**Data:** 2026-11-15
**Base:** `feat/sms-built-in-firmware-catalog-20261115` (PR #905)
**Status:** implementação incremental; boot sem cartucho permanece bloqueado até o dispositivo interno entrar no runtime temporal.
## 1. Objetivo
Disponibilizar, no contexto dos sistemas Master System I e Master System II, uma tela própria de gerenciamento de firmware. O host Slint configura e apresenta os recursos; o catálogo Rust identifica arquivos; o runtime executa o dispositivo efetivamente selecionado.
A UI não decide se a BIOS transfere o controle ao cartucho nem se o firmware interno inicia seu jogo. Essa decisão pertence à execução do firmware e ao mapa de memória/barramento.
## 2. Navegação
- Menu superior **Sistema** continua selecionando SMS1 ou SMS2.
- Para ambos os modelos, o menu Sistema oferece **Firmware...**.
- O item abre uma página/painel dedicado, não um submenu estreito.
- A página usa largura limitada ao viewport, altura limitada à janela e rolagem vertical. Textos descritivos têm quebra de linha; linhas de diagnóstico não dependem de largura fixa.
- O seletor de pasta abre o diálogo nativo de diretório do Windows. Cancelar não altera preferências.
- A varredura é explícita e executada fora da thread da UI e, obrigatoriamente, fora do MachineScheduler.
## 3. Fluxo de descoberta
1. Usuário seleciona uma pasta.
2. O caminho é validado e persistido.
3. Scanner `SmsFirmwareCatalog` percorre recursivamente arquivos regulares sem seguir links simbólicos e sem confiar em nome/extensão.
4. O scanner verifica o tamanho antes de ler o conteúdo e identifica por SHA-256.
5. UI exibe nome catalogado, variante de console, região, jogo incorporado, tamanho, SHA-256 abreviado e estado.
6. A tela lista os firmwares únicos identificados (no máximo oito entradas) e agrega contagens de conteúdo desconhecido, tamanho inválido, erros de leitura e duplicatas, evitando uma lista arbitrariamente longa e clipping.
7. Seletores regionais são habilitados somente quando existe ao menos uma imagem identificada e compatível com o sistema selecionado.
A identidade exibida como encontrada não é confundida com seleção, carregamento ou execução.
## 4. Regiões: preferência, território e vídeo
A preferência persistida do usuário tem seis valores:
- `auto`
- `brazil`
- `europe`
- `japan`
- `korea`
- `usa`
A preferência regional não é o mesmo que o sinal elétrico de território ou o padrão de vídeo:
| Preferência | Território SMS | Padrão | Observação |
|---|---|---|---|
| Brasil | Export | PAL-M | Clock NTSC-like |
| USA | Export | NTSC | Export |
| Europa | Export | PAL | Export |
| Japão | Japan | NTSC | Japan |
| Coreia | Japan | NTSC | A detecção física SMS não separa Coreia de Japão; manter a identidade Coreia para seleção de firmware/perfil |
| Automático | Resolvido antes da construção da máquina | Derivado da região resolvida | Nunca passar `auto` ao worker |
SMS Power descreve a detecção de hardware como duas dimensões: Japan/Export e PAL/NTSC/PAL-M. Japan e Korea compartilham a combinação Japanese + NTSC; Europe usa Export + PAL; USA e Brazil usam Export + NTSC/PAL-M. Portanto, Korea deve ser um perfil de software distinto, mas não inventar um quinto estado elétrico de território.
## 5. Política automática de região
A política automática é determinística e executada uma vez antes de construir a máquina:
1. Preferência manual sempre prevalece.
2. Se o DAT exato classificar o jogo como `World`, resolver USA, independentemente do header.
3. Se o DAT exato indicar múltiplas regiões, aplicar a prioridade fixa: Brasil → USA → Europa → Japão → Coreia.
4. Uma identidade DAT única `Korea` resolve Coreia antes do header Japan, pois o sinal físico não distingue as duas regiões.
5. Preservar a política determinística da PR #287: header Japan seleciona Japão; header Export é refinado por uma única região DAT quando disponível.
6. Sem header utilizável, uma única região DAT determina a preferência.
7. Sem evidência suficiente, fallback USA e registrar a origem da decisão.
A resolução é executada no construtor do `SmsTemporalWorker`, antes de criar a thread da máquina. A GUI entrega somente a preferência (`auto` ou uma região manual); não escolhe o território final. A leitura do DAT e o hash acontecem fora do Scheduler.

O cabeçalho TMR SEGA codifica SMS Japan (3) ou SMS Export (4); não diferencia Brasil, Europa, USA ou Coreia. Essas distinções dependem de metadados exatos e/ou preferência manual. CRC isolado não autoriza uma classificação regional.
## 6. Seleção de firmware por região
Cada região mantém sua própria seleção persistente. O scanner pode encontrar múltiplos firmwares para uma região. A seleção deve listar apenas imagens identificadas e compatíveis com o modelo de console selecionado.
O catálogo tipado declara `supported_regions` em cada descritor. A interface deriva a disponibilidade dos seletores dessa metadata e não mantém uma segunda matriz regional hardcoded.

Mapeamento inicial baseado no catálogo tipado:
- Brasil: Hang On & Safari Hunt v2.4 (a BIOS USA/Europe v1.3 não é classificada como compatível com Brasil sem evidência específica).
- USA: BIOS Export USA/Europe v1.3; Hang On v3.4; Hang On & Safari Hunt v2.4; Missile Defense 3-D v4.4.
- Europa: BIOS Export USA/Europe v1.3; Europe v2.0; Hang On v3.4; Hang On & Safari Hunt v2.4; Missile Defense 3-D v4.4; Sonic Europe (SMS2).
- Japão: Japan v2.1.
- Coreia: Samsung Gam*Boy II / Alex Kidd (variante Samsung SMS2).
Essa matriz é disponibilidade catalogada, não prova de boot. O modelo de console deve filtrar a lista usando `SmsConsoleVariant`; não permitir a seleção de uma imagem de SMS2 como firmware de SMS1 nem o inverso.
## 7. Preferências persistentes
Armazenar na configuração desktop versionada:
- diretório de firmware;
- preferência de boot pela BIOS;
- preferência regional (auto ou uma das cinco regiões);
- seleção de firmware independente para Brasil, USA, Europa, Japão e Coreia.
Persistência é responsabilidade do host/aplicação, não do Scheduler. Alterações na pasta disparam uma nova varredura explícita. Alterar seleção/boot não deve reconfigurar uma sessão em execução silenciosamente: aplicar no próximo power-on/reset seguro.
## 8. Boot e lifecycle
Estados distintos:
- `NotConfigured`
- `DirectoryUnavailable`
- `ScanPending`
- `ScanComplete`
- `FirmwareIdentified`
- `FirmwareSelected`
- `FirmwareLoaded`
- `FirmwareExecuting`
- `BootHandoffObserved`
- `BootValidated`
- `FirmwareError`
O botão Play sem cartucho só pode ser habilitado quando:
1. boot por firmware estiver ligado;
2. região manual/automática tiver sido resolvida;
3. firmware selecionado estiver revalidado por tamanho + SHA-256;
4. o runtime oferecer uma máquina sem cartucho e um dispositivo interno com o mapeamento correto para aquela variante.
**Bloqueio atual:** `SmsTemporalBus::new` e `SmsTemporalMachine::new` ainda exigem `CartridgeImage`; `SmsMemory` guarda cartucho obrigatório. O worker temporal atual não recebe bytes de BIOS/firmware. A UI não deve criar ROM vazia/fictícia nem declarar boot sem cartucho antes da refatoração do contrato de mídia/dispositivo interno e da integração dos mapas por variante.
## 9. Plano incremental
### FM1 — preferências, scanner e UI
- Adicionar diretório e scan assíncrono à UI Slint.
- Exibir resultados tipados e diagnósticos.
- Criar cinco seletores regionais independentes, desabilitados sem firmware válido/compatível.
- Persistir diretório, boot enable, preferência regional e seleções.
- Atualizar o menu regional para Auto/Brasil/Europa/Japão/Coreia/USA.
- Implementar resolver automático em `sega-media` e aceitar Korea no perfil temporal.
### FM2 — contrato de mídia do runtime
- Tornar cartucho opcional sem criar ROM fictícia.
- Introduzir dispositivo de firmware interno distinto do mapper do cartucho.
- Preservar BIOS convencional 8 KiB e boot direto.
- Incluir configuração e estado do dispositivo em save states, reset e worker.
- Nenhuma operação de filesystem dentro do worker/Scheduler.
### FM3 — mapeamento por variante
- Implementar e validar Hang On, Hang On & Safari Hunt e Missile Defense 3-D individualmente.
- Implementar Sonic Europe SMS2 separadamente.
- Auditar Samsung Gam*Boy II antes de ativar a seleção executável.
- Testar hand-off com cartucho e execução interna sem cartucho.
### FM4 — integração final
- Play sem ROM cria máquina sem cartucho somente com firmware executável validado.
- UI diferencia seleção/carregamento/execução/hand-off.
- Regressão boot direto e execução com cartucho.
- CI Linux e validação Windows em etapa autorizada.
## 10. Não objetivos / preservação
- Nenhuma decisão de boot na GUI.
- Nenhum callback por instrução.
- Nenhum acesso a arquivos no Scheduler.
- Nenhuma alteração de Z80, MachineScheduler, VDP ou PSG para acomodar a tela.
- Nenhuma BIOS/ROM proprietária no repositório ou CI.
- Não declarar firmware boot-validado a partir do scan ou de testes de catálogo.

## 11. Validação manual Windows — 15/11/2026

O usuário executou a branch da PR #906 em Windows e reportou:

- **Aprovado:** dimensionamento e rolagem da tela do gerenciador após a correção de layout.
- **Aprovado:** Samsung Gam*Boy II / Coreia é oferecido no SMS2 e não no SMS1, conforme a variante catalogada.
- **Cobertura não catalogada:** não há firmware japonês SMS2 na seleção. O descritor conhecido `Master System Japan v2.1` é SMS1; não o promover para SMS2 por semelhança de região/território. Para adicionar BIOS japonesa SMS2, exigir imagem fornecida localmente, tamanho/hash e evidência de revisão/compatibilidade.
- **Aprovado:** SMS2 inicia e executa `Cosmic Spacehead (Europe) (En,Fr,De,Es)` até gameplay, segundo validação manual do usuário. Isso não valida todos os jogos, todos os modos estendidos ou o boot autônomo de firmware.
- **Ainda não validado:** boot sem cartucho e execução dos jogos incorporados nos firmwares. A disponibilidade na lista não é evidência de mapeamento/boot funcional.

A ausência de opção Coreia em SMS1 é intencional com o catálogo atual; a ausência de BIOS Japão em SMS2 deve ser tratada como lacuna de cobertura do catálogo, não como falha de habilitação da ComboBox, até que exista evidência de firmware japonês compatível com SMS2.

O retorno manual não substitui CI nem teste automatizado. Nenhuma ROM/BIOS foi adicionada ao repositório.
