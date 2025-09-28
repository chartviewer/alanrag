# UVM Verification Methodology

## Chapter 4: UVM Base Classes

### 4.3 The uvm_object Class

The abstract uvm_object class is the base class for all UVM data and hierarchical classes. Its primary role is to define and automate a set of methods for common operations such as create, copy, pack/unpack, compare, print, and record. Classes deriving from uvm_object must implement the pure virtual methods such as create() and get_type_name(). The code below demonstrates a simple example of an AMBA advanced peripheral bus (APB) transfer class definition that does not use the UVM object class definition.

#### Example 4-1 Non-UVM Class Definition

```systemverilog
typedef enum bit {APB_READ, APB_WRITE} apb_direction_enum;
class apb_transfer;
    rand bit [31:0] addr;
    rand bit [31:0] data;
    rand apb_direction_enum direction;
    function void print();
        $display("%s transfer: addr=%h data=%h", direction.name(), addr, data);
    endfunction : print
endclass : apb_transfer
```

The simple example above includes a print() method common to almost all transactions. Most data items require print, copy, compare, pack, unpack, and other utility functions. Leaving it up to the class developer to define the signatures of these services is an obstacle to reuse. The environment integrator will have to learn the signatures (names, parameters, return values) and behaviors of multiple classes coming from different resources. The UVM library solves this issue by introducing the uvm_object base class that defines the signature of these commonly needed services. All objects in the testbench should be directly or indirectly derived from uvm_object. The UVM SystemVerilog class library also includes macros that automatically implement the print, copy, clone, compare, pack, and unpack methods, and more.

#### Example 4-2 APB Transfer Derived from uvm_object

```systemverilog
typedef enum bit {APB_READ, APB_WRITE} apb_direction_enum;
class apb_transfer extends uvm_object;
    rand bit [31:0] addr;
    rand bit [31:0] data;
    rand apb_direction_enum direction;
    // Control field - does not translate into signal data
    rand int unsigned transmit_delay; //delay between transfers

    //UVM automation macros for data items
    `uvm_object_utils_begin(apb_transfer)
        `uvm_field_int(addr, UVM_DEFAULT)
        `uvm_field_int(data, UVM_DEFAULT)
        `uvm_field_enum(apb_direction_enum, direction, UVM_DEFAULT)
        `uvm_field_int(transmit_delay, UVM_DEFAULT | UVM_NOCOMPARE)
    `uvm_object_utils_end

    // Constructor - required UVM syntax
    function new (string name="apb_transfer");
        super.new(name);
    endfunction : new
endclass : apb_transfer
```

Lines 9-14: The UVM automation macros

Lines 16-18: The constructor is not mandatory in data objects. If the constructor is used, it must have defaults for all of the arguments.

## Chapter 5: UVM Components

UVM components form the structural backbone of the testbench hierarchy. This chapter covers the various component types and their relationships.

### 5.1 Component Hierarchy

Components are organized in a hierarchical tree structure that mirrors the DUT organization and provides clear communication paths.