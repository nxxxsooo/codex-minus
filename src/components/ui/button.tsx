import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex shrink-0 items-center justify-center gap-[8px] whitespace-nowrap rounded-[2px] border-2 border-border text-[13px] leading-[20px] font-semibold transition-colors duration-150 ease-out focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-45 [&_svg]:shrink-0 [&_svg]:size-[16px]",
  {
    variants: {
      variant: {
        default: "bg-primary text-primary-foreground hover:bg-primary/90",
        secondary: "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        outline: "bg-transparent text-secondary-foreground hover:bg-accent hover:text-accent-foreground",
        ghost: "border-transparent hover:border-border hover:bg-accent hover:text-accent-foreground",
      },
      size: {
        default: "h-[36px] px-[12px] py-0",
        sm: "h-[32px] px-[10px] py-0",
        icon: "size-[32px] p-0",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return <Comp className={cn(buttonVariants({ variant, size, className }))} aria-label={size === "icon" ? props.title : undefined} data-control-size={size ?? "default"} ref={ref} {...props} />;
  },
);
Button.displayName = "Button";

export { Button, buttonVariants };
