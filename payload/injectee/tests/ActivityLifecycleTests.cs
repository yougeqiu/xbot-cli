using System;
using System.IO;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using System.Text.Json;

namespace XbotInjectee;
public static class ActivityLifecycleTests
{
    public enum Change { ActivityInstalled, ActivityUpdated }
    public sealed class Workspace
    {
        public readonly List<string> Calls = new();
        public string Pin = "activity_test==1";
        private bool changed;
        public void NotifyActivityAdd(string code, string version) { Calls.Add("notify"); }
        public void RaiseWorkspaceChanged(Change kind, bool regenerate) { Calls.Add(kind.ToString()); changed = regenerate; }
        public void Save() { Calls.Add("save"); if (changed) Pin = "activity_test==2"; }
        private Task<bool> WaitPythonTaskCenterIdle(CancellationToken token) { Calls.Add("wait"); return Task.FromResult(true); }
    }
    private sealed class Engine
    {
        public string EngineStatus { get; set; } = "None";
        private bool IsRunning => false;
        public bool ProcessIsPaused { get; set; }
    }
    private static void Assert(bool condition, string message) { if (!condition) throw new Exception(message); }
    private static void Reject(Action action)
    {
        try { action(); } catch (ArgumentException) { return; } catch (InvalidOperationException) { return; }
        throw new Exception("unsafe metadata accepted");
    }
    public static string Run()
    {
        ActivityInstallGuard.RequireIdle(new Engine());
        Reject(()=>ActivityInstallGuard.RequireIdle(new Engine{EngineStatus="Running"}));
        Reject(()=>ActivityInstallGuard.RequireIdle(new Engine{ProcessIsPaused=true}));
        Reject(()=>ActivityInstallGuard.RequireIdle(new object()));
        foreach (var upgrade in new[]{false, true})
        {
            var workspace = new Workspace();
            ActivityInstallGuard.NotifySaveAndWait(workspace, "activity_test", "2", upgrade, CancellationToken.None).GetAwaiter().GetResult();
            Assert(string.Join(",", workspace.Calls) == "notify," + (upgrade ? "ActivityUpdated" : "ActivityInstalled") + ",save,wait", "installation notification/save/wait order changed");
            Assert(workspace.Pin == "activity_test==2", "unchanged document retained stale dependency pin");
            ActivityInstallGuard.RequireConsistent("activity_test","2",new[]{"activity_test==1"},new[]{workspace.Pin},new[]{workspace.Pin},"2");
        }
        Reject(()=>ActivityInstallGuard.RequireConsistent("activity_test","2",new[]{"activity_other==7"},new[]{"activity_test==2"},new[]{"activity_test==2"},"2"));
        var root=Path.Combine(Path.GetTempPath(),"xbot-activity-test-"+Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(Path.Combine(root,".dev"));
        const string id="11111111-1111-1111-1111-111111111111";
        var original="{\"uuid\":\""+id+"\",\"robot_type\":\"activity\",\"activity_code\":\"activity_test\"}";
        try
        {
            File.WriteAllText(Path.Combine(root,"package.json"),original);
            using var bad=JsonDocument.Parse("{\"main.py\":\"bad\"}");
            Reject(()=>ActivityMetadata.ApplyFiles(id,root,bad.RootElement));
            using var mixed=JsonDocument.Parse("{\"prototype.block.json\":{},\".dev/../evil.flow.json\":{}}");
            Reject(()=>ActivityMetadata.ApplyFiles(id,root,mixed.RootElement));
            Assert(!File.Exists(Path.Combine(root,"prototype.block.json")),"metadata validation allowed a partial write");
            using var changed=JsonDocument.Parse("{\"package.json\":{\"uuid\":\""+id+"\",\"robot_type\":\"activity\",\"activity_code\":\"activity_other\"}}");
            Reject(()=>ActivityMetadata.ApplyFiles(id,root,changed.RootElement));
            using var valid=JsonDocument.Parse("{\"prototype.block.json\":{\"blocks\":[]},\"package.json\":"+original+"}");
            var result=ActivityMetadata.ApplyFiles(id,root,valid.RootElement);
            Assert(result.ok,"metadata write failed");
            using var data=JsonDocument.Parse(JsonSerializer.Serialize(result.data));
            var backup=data.RootElement.GetProperty("backup").GetString();
            Assert(File.ReadAllText(Path.Combine(backup,"package.json"))==original,"original metadata not backed up");
            Assert(data.RootElement.GetProperty("verification").GetString()=="disk_readback","missing metadata readback");
            Directory.Delete(backup,true);
        }
        finally { Directory.Delete(root,true); }
        return "PASS: install completion ordering, dependency consistency, metadata prevalidation, identity, backup and disk readback";
    }
}
