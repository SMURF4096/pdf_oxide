// Search for a term across all pages of a PDF and print matches.
// Run: dotnet run -- document.pdf "query"

using PdfOxide.Core;

if (args.Length < 2)
{
    Console.Error.WriteLine("Usage: dotnet run -- <file.pdf> \"query\"");
    return 1;
}

var path = args[0];
var query = args[1];

using var doc = PdfDocument.Open(path);
var pages = doc.PageCount;
Console.WriteLine($"Searching for \"{query}\" in {path} ({pages} pages)...\n");

var total = 0;
var pagesWithHits = 0;

for (int i = 0; i < pages; i++)
{
    var results = doc.SearchPage(i, query);
    if (results.Count == 0) continue;

    pagesWithHits++;
    Console.WriteLine($"Page {i + 1}: {results.Count} match(es)");
    foreach (var r in results)
    {
        Console.WriteLine($"  - \"...{r.Context}...\"");
        total++;
    }
    Console.WriteLine();
}

Console.WriteLine($"Found {total} total matches across {pagesWithHits} pages.");
return 0;
