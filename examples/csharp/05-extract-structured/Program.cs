// Extract words with bounding boxes and tables from a PDF page.
// Run: dotnet run --project csharp/ -- document.pdf

using PdfOxide.Core;

if (args.Length < 1)
{
    Console.Error.WriteLine("Usage: dotnet run -- <file.pdf>");
    return 1;
}

var path = args[0];
using var doc = PdfDocument.Open(path);
Console.WriteLine($"Opened: {path}");

var page = 0;

// Words with position data
var words = doc.ExtractWords(page);
Console.WriteLine($"\n--- Words (page {page + 1}) ---");
foreach (var (text, x, y, width, height) in words.Take(20))
{
    Console.WriteLine($"{"\"" + text + "\"",-20} x={x,-7:F1} y={y,-7:F1} w={width,-7:F1} h={height,-7:F1}");
}
if (words.Length > 20)
    Console.WriteLine($"... ({words.Length - 20} more words)");

// Tables
var tables = doc.ExtractTables(page);
Console.WriteLine($"\n--- Tables (page {page + 1}) ---");
if (tables.Length == 0)
    Console.WriteLine("(no tables found)");
foreach (var (rowCount, colCount) in tables)
{
    Console.WriteLine($"Table: {rowCount} rows x {colCount} cols");
}

return 0;
