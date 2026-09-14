# Configurando o sync com Google Drive

O sync é **opcional**. Sem as credenciais o app compila e funciona normalmente;
apenas o botão de sincronizar reporta que não está configurado.

Este repositório não contém credenciais do Google, e não deve conter. Cada
pessoa que compila usa as suas.

## Por que as credenciais não estão no código

Um cliente OAuth do tipo *Desktop app* recebe um `client_secret`, mas ele não é
confidencial de verdade: acompanha cada binário distribuído, e o próprio Google
documenta isso. O problema de commitá-lo não é alguém "roubar o segredo" — é
que, num repositório público, ele é raspado por bots e passa a ser usado para
montar telas de consentimento de phishing **em nome deste app**. O Google pode
suspender o projeto por isso.

O que realmente protege a troca do código de autorização é o PKCE (RFC 7636),
que este app usa: o `code_verifier` é sorteado a cada tentativa e nunca sai do
processo, então interceptar o redirect não basta para resgatar o código.

## Criando o cliente OAuth

1. Acesse o [Google Cloud Console](https://console.cloud.google.com/) e crie
   (ou selecione) um projeto.
2. Em **APIs e Serviços → Biblioteca**, ative a **Google Drive API**.
3. Em **Tela de permissão OAuth**, configure o app. Enquanto estiver em
   *Testing*, adicione sua conta em **Test users**.
4. Em **Credenciais → Criar credenciais → ID do cliente OAuth**, escolha o tipo
   **App para computador** (*Desktop app*).
5. Adicione `http://localhost:8080` como URI de redirecionamento autorizado.

O escopo pedido é `drive.file`, que dá acesso **apenas aos arquivos criados
pelo próprio app** — não ao restante do seu Drive.

## Compilando com as credenciais

As credenciais são lidas em tempo de compilação, via `option_env!`:

```sh
export PLAXO_GOOGLE_CLIENT_ID="...apps.googleusercontent.com"
export PLAXO_GOOGLE_CLIENT_SECRET="..."   # opcional

yarn tauri build
```

`PLAXO_GOOGLE_CLIENT_SECRET` é opcional: é enviado ao endpoint de token quando
presente, para clientes Desktop que exigem, e omitido quando não.

> Como o valor é embutido em tempo de compilação, alterar a variável exige
> recompilar o crate Rust (`cargo clean -p plaxo-otp` se o cache não invalidar).

## Nas releases do CI

Guarde os valores como **secrets do repositório** no GitHub e exponha-os como
variáveis de ambiente no passo de build. Nunca os coloque no `tauri.conf.json`,
em arquivos versionados ou em logs do CI.

```yaml
- name: Build
  env:
    PLAXO_GOOGLE_CLIENT_ID: ${{ secrets.PLAXO_GOOGLE_CLIENT_ID }}
    PLAXO_GOOGLE_CLIENT_SECRET: ${{ secrets.PLAXO_GOOGLE_CLIENT_SECRET }}
  run: yarn tauri build
```

## O que vai para o Drive

O arquivo `plaxo-otp-backup.enc`, que é exatamente o mesmo blob cifrado do
disco local. A chave nunca é enviada: quem tiver acesso ao seu Drive sem a
senha mestre não consegue ler os segredos.
