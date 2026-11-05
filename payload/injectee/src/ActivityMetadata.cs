using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace XbotInjectee;

internal static class ActivityMetadata
{
    private static bool Allowed(string path) =>
        path is "package.json" or "prototype.block.json" or "prototype.activity.xml" or "__init__.py" or ".dev/workspace.state.json" ||
        path.StartsWith(".dev/", StringComparison.Ordinal) && path.EndsWith(".flow.json", StringComparison.Ordinal) &&
        path.Count(c => c == '/') == 1 && !path.Contains('\\') && !path.Contains(':') && !path.Contains("..");

    internal static StudioActions.Result ApplyFiles(string appId, string root, JsonElement files)
    {
        if (!Guid.TryParseExact(appId,"D",out _) || !Path.IsPathFullyQualified(root))
            throw new ArgumentException("appId UUID and absolute package root required");
        root = Path.GetFullPath(root);
        using var original = JsonDocument.Parse(File.ReadAllText(Path.Combine(root,"package.json")));
        var package = original.RootElement;
        if (package.GetProperty("uuid").GetString() != appId || package.GetProperty("robot_type").GetString() != "activity")
            throw new InvalidOperationException("target package identity/type mismatch");
        if (files.ValueKind != JsonValueKind.Object || !files.EnumerateObject().Any())
            throw new ArgumentException("metadata files must be a nonempty object");
        var prepared = new List<(string Relative, string Path, byte[] Bytes, bool Existed)>();
        var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var file in files.EnumerateObject())
        {
            var relative = file.Name;
            if (!Allowed(relative) || !names.Add(relative)) throw new ArgumentException("disallowed or duplicate metadata path: " + relative);
            if (relative == "package.json")
                foreach (var key in new[]{"uuid","robot_type","activity_code"})
                    if (!file.Value.TryGetProperty(key,out var changed) || changed.GetString() != package.GetProperty(key).GetString())
                        throw new InvalidOperationException("metadata cannot change package " + key);
            var path = Path.Combine(root,relative.Replace('/',Path.DirectorySeparatorChar));
            var parent = Path.GetDirectoryName(path);
            if (Directory.Exists(path) || !Directory.Exists(parent) || (File.GetAttributes(parent) & FileAttributes.ReparsePoint) != 0 ||
                File.Exists(path) && (File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0)
                throw new ArgumentException("metadata parent missing or reparse-point target: " + relative);
            string text;
            if (relative.EndsWith(".json",StringComparison.Ordinal))
            {
                if (file.Value.ValueKind != JsonValueKind.Object) throw new ArgumentException("JSON metadata must be an object: " + relative);
                text = JsonSerializer.Serialize(file.Value,new JsonSerializerOptions{WriteIndented=true});
            }
            else
            {
                if (file.Value.ValueKind != JsonValueKind.String) throw new ArgumentException("XML/init metadata must be text: " + relative);
                text = file.Value.GetString();
            }
            prepared.Add((relative,path,new UTF8Encoding(false).GetBytes(text),File.Exists(path)));
        }
        var backup = Path.Combine(Path.GetTempPath(),"xbot-cli-metadata-" + appId + "-" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(backup);
        foreach (var file in prepared.Where(f=>f.Existed))
        {
            var target = Path.Combine(backup,file.Relative);
            Directory.CreateDirectory(Path.GetDirectoryName(target));
            File.Copy(file.Path,target);
        }
        try
        {
            foreach (var file in prepared) File.WriteAllBytes(file.Path,file.Bytes);
            foreach (var file in prepared)
                if (!File.ReadAllBytes(file.Path).SequenceEqual(file.Bytes)) throw new IOException("metadata readback mismatch: " + file.Relative);
        }
        catch (Exception error)
        {
            return new StudioActions.Result(false,"metadata write/readback failed; inspect backup before retrying: " + error.Message,
                new {appId,backup,files=prepared.Select(f=>new{path=f.Relative,existed=f.Existed}).ToArray(),applied="unknown"});
        }
        return new StudioActions.Result(true,"metadata written; reopen and save still required",new {
            appId,backup,files=prepared.Select(f=>f.Relative).ToArray(),verification="disk_readback"});
    }
}
