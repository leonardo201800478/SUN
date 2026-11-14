# R4 — Contrato de identidade física, roteamento e validação de entrada

**Status:** DESIGN BASELINE / IMPLEMENTATION PENDING
**Data:** 2026-11-14
**Base:** `main` em `2ec587629107191f477d6c0d0f5da1a7c2610492`
**Integração SDL3/Slint:** PR #867
**Documentação de estado R4:** PR #868

Este documento define a próxima fatia arquitetural da R4. Não declara aceite de hardware físico nem implementação de identidade persistente.

## 1. Estado auditado

A primeira integração SDL3/Slint está na main. O host `native/sega_desktop/src/input_host.rs`:

- faz polling no timer do host Slint, nominalmente a cada 8 ms;
- atualiza a enumeração após eventos de conexão/desconexão;
- atribui os dois primeiros dispositivos elegíveis aos jogadores por ordem da sessão;
- mapeia Gamepad API e Joystick API para snapshots SMS;
- combina teclado e gamepad por OR no envio das mudanças ao worker;
- neutraliza entradas de gameplay quando não há ROM ou há menu ativo;
- gera solicitação NMI no Start pela borda de subida observada no timer.

O backend `native/input_runtime/src/sdl3_backend.rs`:

- usa ABI SDL3 dinâmica no Windows;
- mantém handles por ID SDL transitório;
- emite DeviceId sequencial por conexão;
- preenche atualmente `DeviceDescriptor.physical_key = None`;
- consulta hotplug pelo scan periódico de 250 ms.

O contrato `PhysicalDeviceKey` já existe no `sega-input-runtime`, mas não está populado pelo SDL3.

O CI #7512 validou a integração de software sem gamepad físico. O CI #7514 validou os gates aplicáveis da PR documental. Nenhum dos dois representa teste físico de controle.

## 2. Modelo de identidade recomendado

Manter duas identidades independentes:

### 2.1 DeviceId — identidade da conexão

- Único durante a vida do backend/processo.
- Emitido em cada conexão.
- Nunca serializado em preferências.
- Referenciado por leituras, eventos e handles ativos.
- Invalidado na desconexão.

### 2.2 PhysicalDeviceKey — identidade persistente

- 128 bits opacos, derivados de uma identidade canônica disponível na plataforma.
- Gerada com hash criptográfico/truncado de um material canônico versionado.
- Nunca incluir caminho bruto, serial ou identificador pessoal em logs.
- Não usar apenas VID/PID, nome amigável ou índice de enumeração.
- A versão do algoritmo/esquema deve permitir migração futura.

Para SDL3/Windows, o próximo spike deve auditar disponibilidade e estabilidade dos identificadores SDL3 (GUID, serial e caminho do dispositivo, conforme versão/API) e sua capacidade de distinguir duas unidades idênticas. A seleção das funções da ABI deve ser feita contra a documentação e headers da versão SDL fixada, sem presumir que um GUID de modelo distingue instâncias físicas.

### 2.3 Identidade ambígua

Se não houver identificador confiável por instância:

- não fabricar uma chave supostamente única a partir de VID/PID + nome;
- classificar a identidade como ambígua/não persistente;
- permitir perfil padrão e associação manual temporária;
- preservar configurações existentes;
- comunicar que reconexão automática exata não é garantida.

O contrato pode evoluir para distinguir explicitamente `Stable`, `SessionOnly` e `Ambiguous`, mas não mudar `PhysicalDeviceKey` sem avaliar compatibilidade pública do crate e todos os consumidores.

## 3. Perfis e roteamento P1/P2

Separar quatro conceitos:

1. **Dispositivo físico:** identidade e capacidades do periférico.
2. **Perfil físico:** bindings, calibração, deadzone e parâmetros daquele dispositivo.
3. **Atribuição:** associação configurável do dispositivo ao Player 1 ou Player 2.
4. **Estado lógico:** snapshot atual entregue ao adaptador SMS.

O perfil acompanha o periférico; a atribuição acompanha a preferência do usuário. Trocar o controle de P1 não deve transferir o perfil de P1 ao novo dispositivo.

Para dois controles idênticos:

- serial/path confiável diferencia instâncias;
- sem esse identificador, a UI deve mostrar ambiguidade e não alegar persistência por instância;
- usuário pode associar manualmente a unidade conectada ao slot naquela sessão;
- remoção do dispositivo libera seu estado, sem mover automaticamente o controle de P2 para P1, salvo política explícita do usuário.

Durante desconexão:

- o snapshot do jogador afetado deve tornar-se neutro;
- não preservar botões pressionados;
- evento de desconexão não deve alterar estado temporal ou avançar o Scheduler;
- reconexão deve restaurar perfil e slot somente quando a política de identidade permitir.

## 4. Persistência e migração

Usar arquivo de preferências do desktop já existente, com seção de input versionada. Não introduzir um segundo arquivo de configuração sem necessidade.

Formato lógico sugerido:

- schema_version;
- profiles keyed by stable physical identity;
- player assignments keyed by physical identity;
- default profile para dispositivos sem identidade persistente;
- calibração por eixo e metadata de capacidades;
- bindings identificados por semantic action e control path.

Migração:

1. Ler o formato atual sem modificá-lo.
2. Converter o mapeamento legado por jogador para perfis default de P1/P2.
3. Não atribuir esses perfis automaticamente a um hardware ambíguo como se fosse a mesma unidade.
4. Escrever a versão nova atomicamente (temporário + flush + rename).
5. Manter backup/rollback durante a primeira migração.
6. Ignorar campos desconhecidos e validar limites de arrays, índices, thresholds e tamanho do arquivo.
7. Preservar todas as preferências de vídeo/áudio/desktop existentes.

Não implementar UI completa antes de estabilizar este contrato e seus testes de round-trip/migração.

## 5. Contrato de captura analógica

Manter três estágios independentes:

- **Raw capture:** registrar excursão física dos eixos/triggers sem aplicar threshold de gameplay.
- **Calibration:** medir centro, mínimo, máximo, ruído e faixa útil por eixo.
- **Gameplay mapping:** aplicar deadzone e threshold do perfil para produzir ações digitais/analógicas.

O threshold de captura deve ser distinto do threshold atual de gameplay (10.000). Triggers unipolares devem ser tratados como faixa 0..32767. Testar ruído no repouso, excursão gradual, intenção, retorno ao centro e trigger parcialmente pressionado.

## 6. Start/NMI e Pause do frontend

No desktop Slint atual:

- Start de gamepad é lido como nível no polling;
- o host detecta borda de subida e chama `SmsTemporalWorker::request_nmi()`;
- F5 não é Pause/NMI: o comando Pause do frontend suspende o worker;
- quando menu está ativo ou não há ROM, o gameplay é neutralizado.

Testes devem cobrir:
- um único NMI por pressionamento contínuo;
- novo NMI após release e novo press;
- nenhuma repetição por polling;
- remoção durante Start sem estado preso;
- menu aberto/fechado e foco de janela;
- ROM carregada/trocada/fechada;
- falha SDL3 seguida de teclado funcional.

Não alterar Scheduler, semântica de interrupções, Z80 ou VDP para contornar problemas de host input.

## 7. Plano de validação

### Automatizável (sem hardware)

- identidade canônica determinística;
- colisões/ambiguidade;
- dois dispositivos com VID/PID/nome iguais;
- perfil default e perfil persistente;
- round-trip e migração de preferências;
- atribuição P1/P2 e remoção sem estado preso;
- composição teclado + gamepad;
- diagonais, release, centro analógico e triggers;
- NMI edge-triggered e distinção NMI/Pause frontend;
- erros e desconexões simulados.

### Físico obrigatório (Windows x64)

Registrar modelo/revisão, firmware, USB/Bluetooth, Windows build e versão SDL3. Testar:

- gamepad SDL mapeado;
- joystick genérico, se disponível;
- todos os press/release, diagonais e botões de ação;
- analógico e retorno ao centro;
- dois controles simultâneos;
- remoção/reconexão, incluindo dispositivo de P1;
- mudança de foco, menus e troca/fechamento de ROM;
- Start e comportamento NMI.

Não marcar PASS em testes físicos sem log/evidência do equipamento. Sem o equipamento, estado é BLOCKED ou NOT RUN.

### Política incremental de CI

Alteração isolada de input runtime:
- fmt/Clippy/tests do input runtime;
- Windows SDL3 ABI/input;
- desktop Windows se o adaptador Slint for alterado.

Alteração apenas documental:
- hygiene/documentation checks.

Não rodar CPAL, PSG, VDP, Z80 ou Scheduler stress sem arquivos/dependências afetados.

## 8. Pause/NMI e Road Rash — investigação independente

Road Rash permanece um caso de reprodução/diagnóstico, não um bug já atribuído ao SDL3.

Plano:
1. Confirmar ROM/revisão e região usadas pelo teste local, sem versionar ROM comercial.
2. Fixar sequência inicial determinística.
3. Executar sem input Pause, com um pulso NMI e com pulsos repetidos.
4. Registrar eventos de host (press/release/NMI), snapshots de CPU/VDP e frames em fronteiras coarse-grained.
5. Comparar com ARES ou outro oráculo independente usando a mesma ROM e sequência.
6. Identificar a primeira divergência antes de alterar runtime.
7. Não modificar MachineScheduler, Z80 ou VDP com base apenas em percepção visual.

## 9. Escopo explicitamente fora desta fatia

- Backend XInput direto: decisão/proposta independente; não declarar implementado nem presumir menor latência.
- GameInput, deduplicação entre backends, rumble, force feedback e sensores.
- UI completa de configurações.
- Medição button-to-photon sem instrumentação e equipamento apropriados.
- Alterações de hardware, CPU, VDP, PSG ou Scheduler.

## 10. Critérios de conclusão R4

R4 somente poderá ser encerrada quando:
- contrato e implementação de identidade estiverem testados;
- persistência/migração e atribuição P1/P2 estiverem validadas;
- testes físicos da matriz definida estiverem registrados;
- Start/NMI estiver validado e diferenciado do Pause frontend;
- nenhum estado ficar preso em foco, menus, hotplug ou troca de ROM;
- CI aplicável estiver verde no SHA candidato.

Até então, estado: **R4 — integração de software concluída; aceite funcional/físico e persistência ainda pendentes.**
