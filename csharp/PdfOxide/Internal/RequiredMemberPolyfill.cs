// Polyfills for C# 11 required members on older frameworks

#if NETSTANDARD2_0 || NETSTANDARD2_1 || NET5_0 || NET6_0

namespace System.Runtime.CompilerServices
{
    /// <summary>
    /// Specifies that a type has required members or that a member is required.
    /// </summary>
    [AttributeUsage(AttributeTargets.Class | AttributeTargets.Struct | AttributeTargets.Field | AttributeTargets.Property, AllowMultiple = false, Inherited = false)]
    internal sealed class RequiredMemberAttribute : Attribute
    {
    }

    /// <summary>
    /// Indicates that compiler support for a particular feature is required for the location where this attribute is applied.
    /// </summary>
    [AttributeUsage(AttributeTargets.All, AllowMultiple = true, Inherited = false)]
    internal sealed class CompilerFeatureRequiredAttribute : Attribute
    {
        public CompilerFeatureRequiredAttribute(string featureName)
        {
            FeatureName = featureName;
        }

        public string FeatureName { get; }
        public bool IsOptional { get; init; }

        public const string RefStructs = nameof(RefStructs);
        public const string RequiredMembers = nameof(RequiredMembers);
    }
}

namespace System.Diagnostics.CodeAnalysis
{
    /// <summary>
    /// Specifies that this constructor sets all required members for the current type, and callers
    /// do not need to set any required members themselves.
    /// </summary>
    [AttributeUsage(AttributeTargets.Constructor, AllowMultiple = false, Inherited = false)]
    internal sealed class SetsRequiredMembersAttribute : Attribute
    {
    }
}

#endif
