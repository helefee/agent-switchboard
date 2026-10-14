using System;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Reflection;
using System.Threading.Tasks;
using Microsoft.Win32;

namespace AgentSwitchboard.Installer
{
    internal sealed class InstallResult
    {
        internal string LaunchError { get; set; }
    }

    internal enum InstallPhase
    {
        RuntimeDownload,
        RuntimeInstall,
        EngineExtract,
        EngineInstall,
    }

    /// One progress report on the way to a finished install. `Fraction` is the
    /// global 0..1 position owned by the engine's weights; `Bytes`/`TotalBytes`
    /// carry the measurable sub-step's byte counts (total null when the engine
    /// is writing files whose final size is unknown).
    internal sealed class InstallProgress
    {
        internal double Fraction { get; set; }
        internal InstallPhase Phase { get; set; }
        internal long Bytes { get; set; }
        internal long? TotalBytes { get; set; }
    }

    internal static class InstallerEngine
    {
        private const string PayloadResource = "AgentSwitchboard.Installer.Engine.payload";

        /// Global fraction weights per phase. Measurable sub-steps (runtime
        /// download, engine extraction) map their real byte fraction into their
        /// segment; the engine's file-writing phase has no known final size, so
        /// it advances by a converging curve over the bytes observed in the
        /// destination directory (reference = embedded engine payload size),
        /// capped below 1 until the engine process exits.
        private sealed class Weights
        {
            internal Weights(bool runtimeMissing, long payloadBytes)
            {
                this.runtimeMissing = runtimeMissing;
                this.payloadBytes = payloadBytes;
                ExtractStart = runtimeMissing ? 0.40 : 0.0;
                ExtractEnd = runtimeMissing ? 0.55 : 0.30;
            }

            private readonly bool runtimeMissing;
            private readonly long payloadBytes;
            private readonly double ExtractStart;
            private readonly double ExtractEnd;
            private const double EngineCap = 0.99;

            internal InstallProgress RuntimeDownload(long bytes, long total)
            {
                return Segment(InstallPhase.RuntimeDownload, 0.0, 0.40, bytes, total);
            }

            internal InstallProgress RuntimeInstall()
            {
                return new InstallProgress { Fraction = 0.40, Phase = InstallPhase.RuntimeInstall };
            }

            internal InstallProgress Extract(long bytes, long total)
            {
                return Segment(InstallPhase.EngineExtract, ExtractStart, ExtractEnd, bytes, total);
            }

            internal InstallProgress Engine(long bytes)
            {
                return new InstallProgress
                {
                    Fraction = ExtractEnd + (EngineCap - ExtractEnd) * (1.0 - Math.Exp(-bytes / (double)Math.Max(1, payloadBytes))),
                    Phase = InstallPhase.EngineInstall,
                    Bytes = bytes,
                };
            }

            private static InstallProgress Segment(
                InstallPhase phase, double start, double end, long bytes, long total)
            {
                double sub = total > 0 ? Math.Max(0.0, Math.Min(1.0, bytes / (double)total)) : 0.0;
                return new InstallProgress
                {
                    Fraction = start + (end - start) * sub,
                    Phase = phase,
                    Bytes = bytes,
                    TotalBytes = total,
                };
            }
        }

        private enum InstalledDirectoryState
        {
            Present,
            Missing,
            Unknown,
        }

        internal static string DetectDirectory()
        {
            if (InstallerProductMetadata.UsesMsiEngine)
            {
                // The WiX engine installs per-machine under Program Files; the
                // Windows Installer uninstall entry is the only live source.
                string located = TryReadMsiInstallLocation();
                if (!String.IsNullOrWhiteSpace(located)) return located;
                return Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
                    InstallerProductMetadata.ProductName);
            }
            string installed = TryReadInstalledDirectory();
            if (!String.IsNullOrWhiteSpace(installed))
            {
                switch (GetInstalledDirectoryState(installed))
                {
                    case InstalledDirectoryState.Present:
                        return installed;
                    case InstalledDirectoryState.Unknown:
                        return installed;
                    default:
                        RemoveStaleInstalledDirectory();
                        break;
                }
            }
            return Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                InstallerProductMetadata.ProductName);
        }

        internal static async Task<InstallResult> Run(
            InstallationRequest request,
            InstallerInvocation invocation,
            IProgress<InstallProgress> progress)
        {
            string temporary = CreateTemporaryDirectory();
            try
            {
                bool runtimeMissing = !HasWebViewRuntime();
                var weights = new Weights(runtimeMissing, PayloadLength());
                if (runtimeMissing) await EnsureWebViewRuntime(temporary, progress, weights);
                string engine = await ExtractEngine(temporary, progress, weights);
                int exitCode = await RunEngine(engine, request, progress, weights);
                if (exitCode != 0 && !(InstallerProductMetadata.UsesMsiEngine && exitCode == 3010))
                    throw new InstallerException(
                        InstallerFailureKind.EngineExecution,
                        "The installation engine returned a non-zero exit code.",
                        null,
                        exitCode);

                if (progress != null) progress.Report(new InstallProgress { Fraction = 1.0, Phase = InstallPhase.EngineInstall });
                var result = new InstallResult();
                if (invocation.RestartAfterInstall)
                {
                    try { Launch(request.Directory, invocation.ApplicationArguments()); }
                    catch (Exception error)
                    {
                        result.LaunchError = error.Message;
                        Trace.TraceWarning("Application restart: " + error.Message);
                    }
                }
                return result;
            }
            finally
            {
                CleanupTemporaryDirectory(temporary);
            }
        }

        internal static void Launch(string directory)
        {
            Launch(directory, String.Empty);
        }

        internal static bool IsInstalledDirectory(string directory)
        {
            return GetInstalledDirectoryState(directory) == InstalledDirectoryState.Present;
        }

        private static InstalledDirectoryState GetInstalledDirectoryState(string directory)
        {
            if (String.IsNullOrWhiteSpace(directory)) return InstalledDirectoryState.Missing;
            try
            {
                string application = Path.Combine(
                    directory,
                    InstallerProductMetadata.ApplicationFileStem + ".exe");
                FileAttributes attributes = File.GetAttributes(application);
                return (attributes & FileAttributes.Directory) == 0
                    ? InstalledDirectoryState.Present
                    : InstalledDirectoryState.Missing;
            }
            catch (FileNotFoundException)
            {
                return InstalledDirectoryState.Missing;
            }
            catch (DirectoryNotFoundException)
            {
                return InstalledDirectoryState.Missing;
            }
            catch (ArgumentException)
            {
                return InstalledDirectoryState.Missing;
            }
            catch (NotSupportedException)
            {
                return InstalledDirectoryState.Missing;
            }
            catch (PathTooLongException)
            {
                return InstalledDirectoryState.Missing;
            }
            catch (IOException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return InstalledDirectoryState.Unknown;
            }
            catch (UnauthorizedAccessException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return InstalledDirectoryState.Unknown;
            }
            catch (System.Security.SecurityException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return InstalledDirectoryState.Unknown;
            }
        }

        private static string RegistryPath
        {
            get
            {
                return "Software\\" + InstallerProductMetadata.ApplicationFileStem
                    + "\\" + InstallerProductMetadata.ProductName;
            }
        }

        private static string CreateTemporaryDirectory()
        {
            try
            {
                string temporary = Path.Combine(
                    Path.GetTempPath(),
                    "agent-switchboard-setup-" + Guid.NewGuid().ToString("N"));
                System.IO.Directory.CreateDirectory(temporary);
                return temporary;
            }
            catch (IOException error)
            {
                throw TemporaryWorkspace(error);
            }
            catch (UnauthorizedAccessException error)
            {
                throw TemporaryWorkspace(error);
            }
            catch (System.Security.SecurityException error)
            {
                throw TemporaryWorkspace(error);
            }
        }

        private static InstallerException TemporaryWorkspace(Exception error)
        {
            return new InstallerException(
                InstallerFailureKind.TemporaryWorkspace,
                "The installer could not create its temporary workspace.",
                error,
                null);
        }

        private static string TryReadInstalledDirectory()
        {
            try
            {
                using (var hive = RegistryKey.OpenBaseKey(RegistryHive.CurrentUser, RegistryView.Registry64))
                using (var key = hive.OpenSubKey(RegistryPath))
                    return key == null ? null : key.GetValue(null) as string;
            }
            catch (IOException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return null;
            }
            catch (UnauthorizedAccessException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return null;
            }
            catch (System.Security.SecurityException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return null;
            }
        }

        /// Windows Installer owns the uninstall entry of the per-machine MSI;
        /// the display name is the stable join key the WiX package carries.
        private static string TryReadMsiInstallLocation()
        {
            try
            {
                const string uninstall = @"Software\Microsoft\Windows\CurrentVersion\Uninstall";
                using (var hive = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, RegistryView.Registry64))
                using (var root = hive.OpenSubKey(uninstall))
                {
                    if (root == null) return null;
                    foreach (var subKeyName in root.GetSubKeyNames())
                    {
                        using (var key = root.OpenSubKey(subKeyName))
                        {
                            if (key == null) continue;
                            if (!String.Equals(
                                    key.GetValue("DisplayName") as string,
                                    InstallerProductMetadata.ProductName,
                                    StringComparison.OrdinalIgnoreCase)) continue;
                            string location = key.GetValue("InstallLocation") as string;
                            if (!String.IsNullOrWhiteSpace(location)) return location;
                        }
                    }
                }
                return null;
            }
            catch (IOException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return null;
            }
            catch (UnauthorizedAccessException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return null;
            }
            catch (System.Security.SecurityException error)
            {
                Trace.TraceWarning("Installer directory lookup: " + error.Message);
                return null;
            }
        }

        private static void RemoveStaleInstalledDirectory()
        {
            try
            {
                using (var hive = RegistryKey.OpenBaseKey(RegistryHive.CurrentUser, RegistryView.Registry64))
                    hive.DeleteSubKeyTree(RegistryPath, false);
            }
            catch (IOException error)
            {
                Trace.TraceWarning("Installer stale-directory cleanup: " + error.Message);
            }
            catch (UnauthorizedAccessException error)
            {
                Trace.TraceWarning("Installer stale-directory cleanup: " + error.Message);
            }
            catch (System.Security.SecurityException error)
            {
                Trace.TraceWarning("Installer stale-directory cleanup: " + error.Message);
            }
        }

        private static long PayloadLength()
        {
            using (var input = Assembly.GetExecutingAssembly().GetManifestResourceStream(PayloadResource))
            {
                if (input == null)
                    throw new InvalidDataException("Installer engine resource is missing.");
                return input.Length;
            }
        }

        private static async Task<string> ExtractEngine(
            string temporary,
            IProgress<InstallProgress> progress,
            Weights weights)
        {
            // msiexec rejects a database whose file extension is not .msi.
            string engine = Path.Combine(
                temporary,
                InstallerProductMetadata.UsesMsiEngine ? "engine.msi" : "engine.exe");
            try
            {
                using (var input = Assembly.GetExecutingAssembly().GetManifestResourceStream(PayloadResource))
                {
                    if (input == null)
                        throw new InvalidDataException("Installer engine resource is missing.");
                    long total = input.Length;
                    long nextReport = 0;
                    long copied = 0;
                    var buffer = new byte[81920];
                    using (var output = File.Create(engine))
                    {
                        int read;
                        while ((read = await input.ReadAsync(buffer, 0, buffer.Length)) > 0)
                        {
                            await output.WriteAsync(buffer, 0, read);
                            copied += read;
                            if (progress != null && copied >= nextReport)
                            {
                                nextReport = copied + ReportGranularity;
                                progress.Report(weights.Extract(copied, total));
                            }
                        }
                    }
                    if (progress != null && total > 0) progress.Report(weights.Extract(total, total));
                }
                return engine;
            }
            catch (Exception error)
            {
                throw new InstallerException(
                    InstallerFailureKind.EngineLaunch,
                    "The installation engine could not be prepared.",
                    error,
                    null);
            }
        }

        private const int ReportGranularity = 512 * 1024;

        private static async Task EnsureWebViewRuntime(
            string temporary,
            IProgress<InstallProgress> progress,
            Weights weights)
        {
            if (HasWebViewRuntime()) return;

            try
            {
                string bootstrapper = Path.Combine(temporary, "MicrosoftEdgeWebview2Setup.exe");
                ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
                using (var client = new WebClient())
                {
                    if (progress != null)
                        client.DownloadProgressChanged += delegate(object sender, DownloadProgressChangedEventArgs eventArgs)
                        {
                            progress.Report(weights.RuntimeDownload(eventArgs.BytesReceived, eventArgs.TotalBytesToReceive));
                        };
                    await client.DownloadFileTaskAsync(
                        new Uri("https://go.microsoft.com/fwlink/p/?LinkId=2124703"),
                        bootstrapper);
                }

                if (progress != null) progress.Report(weights.RuntimeInstall());
                await RunProcess(bootstrapper, "/silent /install", delegate(ProcessStartInfo start)
                {
                    start.UseShellExecute = false;
                    start.CreateNoWindow = true;
                });
                if (!HasWebViewRuntime())
                    throw new IOException("WebView2 installation did not complete.");
            }
            catch (Exception error)
            {
                throw new InstallerException(
                    InstallerFailureKind.WebViewRuntime,
                    "Microsoft Edge WebView2 Runtime could not be prepared.",
                    error,
                    null);
            }
        }

        /// One engine invocation. The command lines of both engines live here
        /// and nowhere else: NSIS runs the extracted setup directly, the MSI
        /// engine is driven through an elevated msiexec. While the engine
        /// writes files, its destination directory is polled so the ring keeps
        /// moving on real bytes; the final size is unknown, so the reported
        /// fraction converges and never pretends completion before the exit.
        private static async Task<int> RunEngine(
            string engine,
            InstallationRequest request,
            IProgress<InstallProgress> progress,
            Weights weights)
        {
            // Updates may delete before they write; the ring only ever moves
            // forward, so the reported size is the peak observed so far.
            long peak = 0;
            var heartbeat = progress == null ? (Action)null : delegate
            {
                long bytes = DestinationBytes(request.Directory);
                if (bytes > peak) peak = bytes;
                if (peak > 0) progress.Report(weights.Engine(peak));
            };

            if (!InstallerProductMetadata.UsesMsiEngine)
                return await RunProcess(
                    engine,
                    BuildNsisArguments(request),
                    delegate(ProcessStartInfo start)
                    {
                        start.UseShellExecute = false;
                        start.CreateNoWindow = true;
                    },
                    heartbeat);

            try
            {
                return await RunProcess(
                    "msiexec.exe",
                    "/i \"" + engine + "\" /qn /norestart",
                    delegate(ProcessStartInfo start)
                    {
                        // The WiX package installs per-machine, so the engine
                        // elevation goes through the shell's UAC prompt.
                        start.UseShellExecute = true;
                        start.Verb = "runas";
                    },
                    heartbeat);
            }
            catch (Exception error)
            {
                if (IsElevationDeclined(error))
                    throw new InstallerException(
                        InstallerFailureKind.ElevationDeclined,
                        "The administrator approval was declined.",
                        error,
                        null);
                throw new InstallerException(
                    InstallerFailureKind.EngineLaunch,
                    "The installation engine could not start.",
                    error,
                    null);
            }
        }

        /// Total bytes of every file below `directory`; -1 when the tree is
        /// not readable (absent, locked, or access denied). Junction loops are
        /// the caller's small flat install trees, not general forests.
        private static long DestinationBytes(string directory)
        {
            try
            {
                if (!Directory.Exists(directory)) return 0;
                long total = 0;
                foreach (string file in Directory.EnumerateFiles(directory, "*", SearchOption.AllDirectories))
                {
                    try { total += (new FileInfo(file)).Length; }
                    catch (IOException) { }
                    catch (UnauthorizedAccessException) { }
                }
                return total;
            }
            catch (IOException error)
            {
                Trace.TraceWarning("Installer destination size: " + error.Message);
                return -1;
            }
            catch (UnauthorizedAccessException error)
            {
                Trace.TraceWarning("Installer destination size: " + error.Message);
                return -1;
            }
        }

        private static string BuildNsisArguments(InstallationRequest request)
        {
            var arguments = new System.Text.StringBuilder("/S");
            if (request.Update) arguments.Append(" /UPDATE");
            arguments.Append(" /D=").Append(request.Directory);
            return arguments.ToString();
        }

        private static bool IsElevationDeclined(Exception error)
        {
            var native = error as System.ComponentModel.Win32Exception;
            return native != null && native.NativeErrorCode == 1223; // ERROR_CANCELLED
        }

        private static Task<int> RunProcess(
            string executable,
            string arguments,
            Action<ProcessStartInfo> configure,
            Action heartbeat = null)
        {
            return Task.Run(() =>
            {
                var start = new ProcessStartInfo(executable, arguments)
                {
                    WorkingDirectory = Path.GetDirectoryName(executable),
                };
                configure(start);
                using (var process = Process.Start(start))
                {
                    if (process == null)
                        throw new IOException("The installer process could not start.");
                    while (!process.WaitForExit(250))
                    {
                        if (heartbeat != null) heartbeat();
                    }
                    return process.ExitCode;
                }
            });
        }

        private static bool HasWebViewRuntime()
        {
            try
            {
                const string client = @"Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
                foreach (var hive in new[] { RegistryHive.LocalMachine, RegistryHive.CurrentUser })
                foreach (var view in new[] { RegistryView.Registry32, RegistryView.Registry64 })
                using (var root = RegistryKey.OpenBaseKey(hive, view))
                using (var key = root.OpenSubKey(client))
                {
                    Version version;
                    if (key != null
                        && Version.TryParse(key.GetValue("pv") as string, out version)
                        && version.Major > 0)
                        return true;
                }
            }
            catch (IOException error)
            {
                Trace.TraceWarning("WebView runtime lookup: " + error.Message);
            }
            catch (UnauthorizedAccessException error)
            {
                Trace.TraceWarning("WebView runtime lookup: " + error.Message);
            }
            catch (System.Security.SecurityException error)
            {
                Trace.TraceWarning("WebView runtime lookup: " + error.Message);
            }
            return false;
        }

        private static void CleanupTemporaryDirectory(string temporary)
        {
            try { System.IO.Directory.Delete(temporary, true); }
            catch (IOException error) { Trace.TraceWarning("Installer staging cleanup: " + error.Message); }
            catch (UnauthorizedAccessException error)
            {
                Trace.TraceWarning("Installer staging cleanup: " + error.Message);
            }
        }

        private static void Launch(string directory, string arguments)
        {
            string executable = Path.Combine(
                directory,
                InstallerProductMetadata.ApplicationFileStem + ".exe");
            var process = Process.Start(new ProcessStartInfo(executable, arguments)
            {
                UseShellExecute = true,
                WorkingDirectory = directory,
            });
            if (process == null) throw new IOException("The application could not start.");
        }
    }
}
