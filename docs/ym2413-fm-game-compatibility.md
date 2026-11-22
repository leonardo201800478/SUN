# Jogos Sega Master System com suporte ao FM (YM2413)

**Registro:** 2026-11-22  
**Origem:** relação de compatibilidade fornecida pelo mantenedor do projeto SUN.  
**Escopo:** versões japonesas de jogos para Sega Master System com suporte ao áudio FM do YM2413.

Esta relação deve orientar a seleção de ROMs para validação de áudio FM real no SUN. A presença de um título nesta lista indica suporte FM na versão/edição indicada pela classificação; não substitui a verificação da revisão exata da ROM, da região, da placa FM e da execução efetiva das rotinas de inicialização do chip.

## 1. Jogos inéditos no Japão com suporte ao FM

- Altered Beast
- California Games
- Casino Games
- Cloud Master
- Galaxy Force
- Game Box Série Esportes Radicais
- Golfamania
- Mônica no Castelo do Dragão
- Out Run 3-D
- Poseidon Wars 3-D
- Rampage
- Rastan
- Rescue Mission
- Scramble Spirits
- Shanghai
- Tennis Ace
- Time Soldiers
- Turma da Mônica em: O Resgate
- Ultima IV
- Vigilante
- Wonder Boy III: The Dragon's Trap

## 2. Jogos lançados no Japão cujas versões ocidentais também suportam FM

- After Burner
- Alex Kidd: The Lost Stars
- Alien Syndrome
- Aztec Adventure
- Blade Eagle 3-D
- Bomber Raid
- Captain Silver
- Cyborg Hunter
- Double Dragon
- Fantasy Zone II
- Fantasy Zone: The Maze
- Global Defense
- Golvellius - Valley of Doom
- Great Golf
- Kenseiden
- Lord of The Sword
- Maze Hunter 3-D
- Miracle Warriors - Seal of The Dark Lord
- Out Run
- Parlour Games
- Penguin Land
- Power Strike
- R-Type
- Shinobi
- Space Harrier 3-D
- SpellCaster
- Thunder Blade
- Wonder Boy in Monster Land
- Zaxxon 3-D
- Zillion II - The Tri Formation

## 3. Exceções importantes: Phantasy Star e Ys

**Phantasy Star** e **Ys** não fazem parte da relação de versões ocidentais com suporte FM.

Segundo a informação registrada pelo mantenedor, o suporte FM foi removido das versões ocidentais para liberar espaço para o texto em inglês da tradução. O texto japonês original, que poderia ser acessado por modificação de região, também foi afetado por essa alteração.

A observação é especialmente relevante para a validação auditiva: esses dois títulos são apontados pelo mantenedor como exemplos em que a música FM é particularmente marcante, mas as ROMs ocidentais não devem ser usadas como prova de compatibilidade FM.

## 4. Uso desta relação nos testes do SUN

1. Priorizar ROMs japonesas identificadas nesta relação para os testes de integração Z80/Scheduler → YM2413 → mixer nativo.
2. Registrar o SHA-256 e a revisão exata de cada ROM utilizada.
3. Não inferir suporte FM apenas pelo nome do jogo: confirmar a variante regional e a revisão da ROM.
4. Distinguir três resultados: jogo executa; jogo efetivamente inicializa/escreve no YM2413; saída FM não silenciosa chega ao mixer e ao dispositivo de áudio.
5. Manter Phantasy Star e Ys ocidentais fora da lista de fixtures de aceitação FM, sem concluir que todas as edições japonesas ou modificadas se comportam da mesma forma.

**Nota de proveniência:** as classificações e observações desta página foram registradas conforme fornecidas pelo mantenedor do SUN; ainda não representam uma auditoria independente de cada ROM/revisão.
