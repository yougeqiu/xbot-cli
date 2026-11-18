using System;
using System.Collections;
using System.Linq;

namespace XbotInjectee;

internal static class FlowParameterTransfer
{
    internal static int CopyToEmpty(IList source, IList target)
    {
        if (ReferenceEquals(source, target) || target.Count != 0)
            throw new InvalidOperationException("target parameters must be empty and distinct from source");
        var copies = source.Cast<object>().Select(value =>
            value is ICloneable cloneable ? cloneable.Clone() : throw new NotSupportedException("parameter is not cloneable")).ToArray();
        if (copies.Any(value => value == null)) throw new InvalidOperationException("parameter clone returned null");
        try
        {
            foreach (var value in copies) target.Add(value);
        }
        catch
        {
            target.Clear();
            throw;
        }
        return copies.Length;
    }
}
