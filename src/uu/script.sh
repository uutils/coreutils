for d in */; do
    folder_name="${d%/}"
    file="$folder_name/locales/en-US.ftl"

    if [ -f "$file" ]; then
        # 'a' добавя съдържанието на нов ред ВЕДНАГА след намерения ред
        sed -i "/-about =/a \  Part of uutils." "$file"
        echo "Добавено 'Part of uutils.' на нов ред в: $file"
    fi
done
