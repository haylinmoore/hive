package main

import (
	"bytes"
	"fmt"
	"syscall/js"

	"hive/web/lambda/sheepda"
)

func main() {
	js.Global().Set("sheepda", js.ValueOf(map[string]any{
		"eval": js.FuncOf(func(this js.Value, args []js.Value) any {
			out, fail := eval(args[0].String(), args[1].String())
			return js.ValueOf([]any{out, fail})
		}),
	}))
	select {}
}

func eval(source, mode string) (output, failure string) {
	defer func() {
		if r := recover(); r != nil {
			output, failure = "", fmt.Sprint(r)
		}
	}()

	prog, err := sheepda.Parse(sheepda.NewStream(bytes.NewReader([]byte(source))))
	if err != nil {
		return "", err.Error()
	}

	if mode == "parse" {
		return fmt.Sprint(prog.Expr), ""
	}

	var out bytes.Buffer
	val, err := sheepda.Eval(sheepda.NewScopeWithBuiltins(&out, nil), prog)
	if err != nil {
		return "", err.Error()
	}

	switch mode {
	case "output":
		return out.String(), ""
	case "result":
		return val.String(), ""
	}
	return "", "undefined mode"
}
