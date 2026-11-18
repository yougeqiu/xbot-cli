using System;
using System.Collections;

namespace XbotInjectee;

public static class FlowParameterTransferTests
{
    private sealed class Parameter : ICloneable
    {
        public string Name;
        public string Type;
        public string Value;
        public object Clone() => MemberwiseClone();
    }

    public static void Run()
    {
        var original = new Parameter {Name="web_page",Type="xbot._web.browser.WebBrowser",Value=null};
        var source = new ArrayList {original, new Parameter {Name="文件名",Type="str",Value="默认名称"}};
        var target = new ArrayList();
        if (FlowParameterTransfer.CopyToEmpty(source,target) != 2) throw new Exception("parameter count changed");
        var copy = (Parameter)target[0];
        if (ReferenceEquals(original,copy) || copy.Type != original.Type || copy.Value != null)
            throw new Exception("parameter clone lost identity/type/null semantics");
        copy.Name="changed";
        if (original.Name!="web_page") throw new Exception("source was changed");
        try {FlowParameterTransfer.CopyToEmpty(source,target); throw new Exception("nonempty target accepted");}
        catch (InvalidOperationException) { }
        try {FlowParameterTransfer.CopyToEmpty(source,source); throw new Exception("same collection accepted");}
        catch (InvalidOperationException) { }
        Console.WriteLine("PASS: parameter clone retains custom type, defaults, order and source isolation");
    }
}
