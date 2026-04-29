// PDF/A conversion — v0.3.40
// Run: dotnet run
using System;
using System.IO;
using PdfOxide.Core;

const string OutDir = "output";
Directory.CreateDirectory(OutDir);

Console.WriteLine("Demonstrating PDF/A conversion...");

using var builder = DocumentBuilder.Create();
builder.Title("PDF/A Demo");
builder.LetterPage()
    .Font("Helvetica", 12)
    .At(72, 720).Heading(1, "PDF/A Document")
    .At(72, 690).Paragraph("Converting to PDF/A-2b for archival.")
    .Done();
byte[] pdfBytes = builder.Build();

using var editor = DocumentEditor.OpenFromBytes(pdfBytes);
try
{
    editor.ConvertToPdfA(PdfALevel.A2b);
    Console.WriteLine("  Converted to PDF/A-2b.");
}
catch (Exception ex)
{
    Console.WriteLine($"  Conversion note: {ex.Message}");
}

byte[] outBytes = editor.SaveToBytes();
Console.WriteLine($"  Output: {outBytes.Length} bytes");

var path = Path.Combine(OutDir, "pdfa.pdf");
File.WriteAllBytes(path, outBytes);
Console.WriteLine($"Written: {path}");
