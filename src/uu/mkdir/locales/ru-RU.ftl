mkdir-about = Создать указанную директорию(и), если они не существуют.
mkdir-usage = mkdir [ПАРАМЕТРЫ]... ДИРЕКТОРИЯ...
mkdir-after-help = Каждый РЕЖИМ (MODE) имеет вид [ugoa]*([-+=]([rwxXst]*|[ugo]))+|[-+=]?[0-7]+.

# Help messages
mkdir-help-mode = установить права доступа (не поддерживается в Windows)
mkdir-help-parents = создать родительские директории при необходимости
mkdir-help-verbose = выводить сообщение для каждой создаваемой директории
mkdir-help-selinux = установить контекст безопасности SELinux для каждой создаваемой директории в тип по умолчанию
mkdir-help-context = как -Z, или если указан CTX, установить контекст безопасности SELinux или SMACK в CTX

# Error messages
mkdir-error-empty-directory-name = невозможно создать директорию '': Нет такого файла или каталога
mkdir-error-file-exists = { $path }: Файл существует
mkdir-error-failed-to-create-tree = не удалось создать полное дерево каталогов
mkdir-error-cannot-set-permissions = невозможно установить права доступа { $path }

# Verbose output
mkdir-verbose-created-directory = { $util_name }: создана директория { $path }