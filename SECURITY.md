# Política de Segurança

## Reportando uma vulnerabilidade

Não abra uma issue pública para falhas de segurança.

Use **[Security Advisories](https://github.com/carloseduardodb/plaxo_otp_pub/security/advisories/new)**
do GitHub, que cria um canal privado. Procuro responder em até 7 dias.

Ao reportar, ajuda muito incluir: a versão afetada, o sistema operacional, os
passos para reproduzir e qual o impacto concreto.

## Escopo

Esta aplicação guarda segredos TOTP — segundo fator de autenticação. Interessam
especialmente relatos sobre:

- Recuperação do cofre sem a senha mestre
- Vazamento de segredos em logs, arquivos temporários, crash dumps ou área de
  transferência
- Problemas no fluxo OAuth do Google Drive
- Falhas na derivação de chave ou na cifragem

## Limitações conhecidas

Estas não são vulnerabilidades — são limites de projeto, documentados no README
em "O que isto não protege":

- Malware rodando com o seu usuário consegue ler a chave da memória enquanto o
  app está destrancado
- Uma senha mestre fraca continua sendo adivinhável offline; o Argon2id
  encarece cada tentativa, não elimina o ataque
- Um código copiado fica até 30 segundos no clipboard, legível por qualquer
  processo nesse intervalo
- **Os binários das releases não são assinados e não há canal de atualização
  automática.** Uma correção de segurança não chega sozinha a quem já instalou:
  é preciso acompanhar as releases e reinstalar. Assinar exige certificado de
  code signing (Windows) e conta Apple Developer (macOS)

## Histórico

Cofres gravados pela versão 1.3.3 ou anterior usavam uma derivação de chave
fraca: um único SHA-256 com salt fixo, compilado no binário. Isso permitia
ataques de dicionário muito rápidos contra quem tivesse acesso ao arquivo
cifrado (inclusive à cópia no Google Drive).

A partir da primeira release pública a derivação é Argon2id com salt aleatório
por cofre, e os cofres antigos são migrados automaticamente no primeiro
desbloqueio. **Se você usou uma versão anterior, considere trocar a senha
mestre** — a migração protege o arquivo a partir de agora, mas não desfaz uma
eventual cópia que já tenha vazado.
