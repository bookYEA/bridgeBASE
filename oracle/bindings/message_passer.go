// Code generated - DO NOT EDIT.
// This file is a generated binding and any manual changes will be lost.

package bindings

import (
	"errors"
	"math/big"
	"strings"

	ethereum "github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/event"
)

// Reference imports to suppress errors if they are not otherwise used.
var (
	_ = errors.New
	_ = big.NewInt
	_ = strings.NewReader
	_ = ethereum.NotFound
	_ = bind.Bind
	_ = common.Big1
	_ = types.BloomLookup
	_ = event.NewSubscription
	_ = abi.ConvertType
)

// MessagePasserAccountMeta is an auto generated low-level Go binding around an user-defined struct.
type MessagePasserAccountMeta struct {
	PubKey     [32]byte
	IsSigner   bool
	IsWritable bool
}

// MessagePasserInstruction is an auto generated low-level Go binding around an user-defined struct.
type MessagePasserInstruction struct {
	ProgramId [32]byte
	Accounts  []MessagePasserAccountMeta
	Data      []byte
}

// MessagePasserMetaData contains all meta data concerning the MessagePasser contract.
var MessagePasserMetaData = &bind.MetaData{
	ABI: "[{\"type\":\"function\",\"name\":\"MESSAGE_VERSION\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"uint16\",\"internalType\":\"uint16\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"initiateWithdrawal\",\"inputs\":[{\"name\":\"ixs\",\"type\":\"tuple[]\",\"internalType\":\"structMessagePasser.Instruction[]\",\"components\":[{\"name\":\"programId\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"},{\"name\":\"accounts\",\"type\":\"tuple[]\",\"internalType\":\"structMessagePasser.AccountMeta[]\",\"components\":[{\"name\":\"pubKey\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"},{\"name\":\"isSigner\",\"type\":\"bool\",\"internalType\":\"bool\"},{\"name\":\"isWritable\",\"type\":\"bool\",\"internalType\":\"bool\"}]},{\"name\":\"data\",\"type\":\"bytes\",\"internalType\":\"bytes\"}]}],\"outputs\":[],\"stateMutability\":\"payable\"},{\"type\":\"function\",\"name\":\"messageNonce\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"uint256\",\"internalType\":\"uint256\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"sentMessages\",\"inputs\":[{\"name\":\"\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"}],\"outputs\":[{\"name\":\"\",\"type\":\"bool\",\"internalType\":\"bool\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"version\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"string\",\"internalType\":\"string\"}],\"stateMutability\":\"view\"},{\"type\":\"event\",\"name\":\"MessagePassed\",\"inputs\":[{\"name\":\"nonce\",\"type\":\"uint256\",\"indexed\":true,\"internalType\":\"uint256\"},{\"name\":\"sender\",\"type\":\"address\",\"indexed\":true,\"internalType\":\"address\"},{\"name\":\"ixs\",\"type\":\"tuple[]\",\"indexed\":false,\"internalType\":\"structMessagePasser.Instruction[]\",\"components\":[{\"name\":\"programId\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"},{\"name\":\"accounts\",\"type\":\"tuple[]\",\"internalType\":\"structMessagePasser.AccountMeta[]\",\"components\":[{\"name\":\"pubKey\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"},{\"name\":\"isSigner\",\"type\":\"bool\",\"internalType\":\"bool\"},{\"name\":\"isWritable\",\"type\":\"bool\",\"internalType\":\"bool\"}]},{\"name\":\"data\",\"type\":\"bytes\",\"internalType\":\"bytes\"}]},{\"name\":\"withdrawalHash\",\"type\":\"bytes32\",\"indexed\":false,\"internalType\":\"bytes32\"}],\"anonymous\":false}]",
}

// MessagePasserABI is the input ABI used to generate the binding from.
// Deprecated: Use MessagePasserMetaData.ABI instead.
var MessagePasserABI = MessagePasserMetaData.ABI

// MessagePasser is an auto generated Go binding around an Ethereum contract.
type MessagePasser struct {
	MessagePasserCaller     // Read-only binding to the contract
	MessagePasserTransactor // Write-only binding to the contract
	MessagePasserFilterer   // Log filterer for contract events
}

// MessagePasserCaller is an auto generated read-only Go binding around an Ethereum contract.
type MessagePasserCaller struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// MessagePasserTransactor is an auto generated write-only Go binding around an Ethereum contract.
type MessagePasserTransactor struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// MessagePasserFilterer is an auto generated log filtering Go binding around an Ethereum contract events.
type MessagePasserFilterer struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// MessagePasserSession is an auto generated Go binding around an Ethereum contract,
// with pre-set call and transact options.
type MessagePasserSession struct {
	Contract     *MessagePasser    // Generic contract binding to set the session for
	CallOpts     bind.CallOpts     // Call options to use throughout this session
	TransactOpts bind.TransactOpts // Transaction auth options to use throughout this session
}

// MessagePasserCallerSession is an auto generated read-only Go binding around an Ethereum contract,
// with pre-set call options.
type MessagePasserCallerSession struct {
	Contract *MessagePasserCaller // Generic contract caller binding to set the session for
	CallOpts bind.CallOpts        // Call options to use throughout this session
}

// MessagePasserTransactorSession is an auto generated write-only Go binding around an Ethereum contract,
// with pre-set transact options.
type MessagePasserTransactorSession struct {
	Contract     *MessagePasserTransactor // Generic contract transactor binding to set the session for
	TransactOpts bind.TransactOpts        // Transaction auth options to use throughout this session
}

// MessagePasserRaw is an auto generated low-level Go binding around an Ethereum contract.
type MessagePasserRaw struct {
	Contract *MessagePasser // Generic contract binding to access the raw methods on
}

// MessagePasserCallerRaw is an auto generated low-level read-only Go binding around an Ethereum contract.
type MessagePasserCallerRaw struct {
	Contract *MessagePasserCaller // Generic read-only contract binding to access the raw methods on
}

// MessagePasserTransactorRaw is an auto generated low-level write-only Go binding around an Ethereum contract.
type MessagePasserTransactorRaw struct {
	Contract *MessagePasserTransactor // Generic write-only contract binding to access the raw methods on
}

// NewMessagePasser creates a new instance of MessagePasser, bound to a specific deployed contract.
func NewMessagePasser(address common.Address, backend bind.ContractBackend) (*MessagePasser, error) {
	contract, err := bindMessagePasser(address, backend, backend, backend)
	if err != nil {
		return nil, err
	}
	return &MessagePasser{MessagePasserCaller: MessagePasserCaller{contract: contract}, MessagePasserTransactor: MessagePasserTransactor{contract: contract}, MessagePasserFilterer: MessagePasserFilterer{contract: contract}}, nil
}

// NewMessagePasserCaller creates a new read-only instance of MessagePasser, bound to a specific deployed contract.
func NewMessagePasserCaller(address common.Address, caller bind.ContractCaller) (*MessagePasserCaller, error) {
	contract, err := bindMessagePasser(address, caller, nil, nil)
	if err != nil {
		return nil, err
	}
	return &MessagePasserCaller{contract: contract}, nil
}

// NewMessagePasserTransactor creates a new write-only instance of MessagePasser, bound to a specific deployed contract.
func NewMessagePasserTransactor(address common.Address, transactor bind.ContractTransactor) (*MessagePasserTransactor, error) {
	contract, err := bindMessagePasser(address, nil, transactor, nil)
	if err != nil {
		return nil, err
	}
	return &MessagePasserTransactor{contract: contract}, nil
}

// NewMessagePasserFilterer creates a new log filterer instance of MessagePasser, bound to a specific deployed contract.
func NewMessagePasserFilterer(address common.Address, filterer bind.ContractFilterer) (*MessagePasserFilterer, error) {
	contract, err := bindMessagePasser(address, nil, nil, filterer)
	if err != nil {
		return nil, err
	}
	return &MessagePasserFilterer{contract: contract}, nil
}

// bindMessagePasser binds a generic wrapper to an already deployed contract.
func bindMessagePasser(address common.Address, caller bind.ContractCaller, transactor bind.ContractTransactor, filterer bind.ContractFilterer) (*bind.BoundContract, error) {
	parsed, err := MessagePasserMetaData.GetAbi()
	if err != nil {
		return nil, err
	}
	return bind.NewBoundContract(address, *parsed, caller, transactor, filterer), nil
}

// Call invokes the (constant) contract method with params as input values and
// sets the output to result. The result type might be a single field for simple
// returns, a slice of interfaces for anonymous returns and a struct for named
// returns.
func (_MessagePasser *MessagePasserRaw) Call(opts *bind.CallOpts, result *[]interface{}, method string, params ...interface{}) error {
	return _MessagePasser.Contract.MessagePasserCaller.contract.Call(opts, result, method, params...)
}

// Transfer initiates a plain transaction to move funds to the contract, calling
// its default method if one is available.
func (_MessagePasser *MessagePasserRaw) Transfer(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _MessagePasser.Contract.MessagePasserTransactor.contract.Transfer(opts)
}

// Transact invokes the (paid) contract method with params as input values.
func (_MessagePasser *MessagePasserRaw) Transact(opts *bind.TransactOpts, method string, params ...interface{}) (*types.Transaction, error) {
	return _MessagePasser.Contract.MessagePasserTransactor.contract.Transact(opts, method, params...)
}

// Call invokes the (constant) contract method with params as input values and
// sets the output to result. The result type might be a single field for simple
// returns, a slice of interfaces for anonymous returns and a struct for named
// returns.
func (_MessagePasser *MessagePasserCallerRaw) Call(opts *bind.CallOpts, result *[]interface{}, method string, params ...interface{}) error {
	return _MessagePasser.Contract.contract.Call(opts, result, method, params...)
}

// Transfer initiates a plain transaction to move funds to the contract, calling
// its default method if one is available.
func (_MessagePasser *MessagePasserTransactorRaw) Transfer(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _MessagePasser.Contract.contract.Transfer(opts)
}

// Transact invokes the (paid) contract method with params as input values.
func (_MessagePasser *MessagePasserTransactorRaw) Transact(opts *bind.TransactOpts, method string, params ...interface{}) (*types.Transaction, error) {
	return _MessagePasser.Contract.contract.Transact(opts, method, params...)
}

// MESSAGEVERSION is a free data retrieval call binding the contract method 0x3f827a5a.
//
// Solidity: function MESSAGE_VERSION() view returns(uint16)
func (_MessagePasser *MessagePasserCaller) MESSAGEVERSION(opts *bind.CallOpts) (uint16, error) {
	var out []interface{}
	err := _MessagePasser.contract.Call(opts, &out, "MESSAGE_VERSION")

	if err != nil {
		return *new(uint16), err
	}

	out0 := *abi.ConvertType(out[0], new(uint16)).(*uint16)

	return out0, err

}

// MESSAGEVERSION is a free data retrieval call binding the contract method 0x3f827a5a.
//
// Solidity: function MESSAGE_VERSION() view returns(uint16)
func (_MessagePasser *MessagePasserSession) MESSAGEVERSION() (uint16, error) {
	return _MessagePasser.Contract.MESSAGEVERSION(&_MessagePasser.CallOpts)
}

// MESSAGEVERSION is a free data retrieval call binding the contract method 0x3f827a5a.
//
// Solidity: function MESSAGE_VERSION() view returns(uint16)
func (_MessagePasser *MessagePasserCallerSession) MESSAGEVERSION() (uint16, error) {
	return _MessagePasser.Contract.MESSAGEVERSION(&_MessagePasser.CallOpts)
}

// MessageNonce is a free data retrieval call binding the contract method 0xecc70428.
//
// Solidity: function messageNonce() view returns(uint256)
func (_MessagePasser *MessagePasserCaller) MessageNonce(opts *bind.CallOpts) (*big.Int, error) {
	var out []interface{}
	err := _MessagePasser.contract.Call(opts, &out, "messageNonce")

	if err != nil {
		return *new(*big.Int), err
	}

	out0 := *abi.ConvertType(out[0], new(*big.Int)).(**big.Int)

	return out0, err

}

// MessageNonce is a free data retrieval call binding the contract method 0xecc70428.
//
// Solidity: function messageNonce() view returns(uint256)
func (_MessagePasser *MessagePasserSession) MessageNonce() (*big.Int, error) {
	return _MessagePasser.Contract.MessageNonce(&_MessagePasser.CallOpts)
}

// MessageNonce is a free data retrieval call binding the contract method 0xecc70428.
//
// Solidity: function messageNonce() view returns(uint256)
func (_MessagePasser *MessagePasserCallerSession) MessageNonce() (*big.Int, error) {
	return _MessagePasser.Contract.MessageNonce(&_MessagePasser.CallOpts)
}

// SentMessages is a free data retrieval call binding the contract method 0x82e3702d.
//
// Solidity: function sentMessages(bytes32 ) view returns(bool)
func (_MessagePasser *MessagePasserCaller) SentMessages(opts *bind.CallOpts, arg0 [32]byte) (bool, error) {
	var out []interface{}
	err := _MessagePasser.contract.Call(opts, &out, "sentMessages", arg0)

	if err != nil {
		return *new(bool), err
	}

	out0 := *abi.ConvertType(out[0], new(bool)).(*bool)

	return out0, err

}

// SentMessages is a free data retrieval call binding the contract method 0x82e3702d.
//
// Solidity: function sentMessages(bytes32 ) view returns(bool)
func (_MessagePasser *MessagePasserSession) SentMessages(arg0 [32]byte) (bool, error) {
	return _MessagePasser.Contract.SentMessages(&_MessagePasser.CallOpts, arg0)
}

// SentMessages is a free data retrieval call binding the contract method 0x82e3702d.
//
// Solidity: function sentMessages(bytes32 ) view returns(bool)
func (_MessagePasser *MessagePasserCallerSession) SentMessages(arg0 [32]byte) (bool, error) {
	return _MessagePasser.Contract.SentMessages(&_MessagePasser.CallOpts, arg0)
}

// Version is a free data retrieval call binding the contract method 0x54fd4d50.
//
// Solidity: function version() view returns(string)
func (_MessagePasser *MessagePasserCaller) Version(opts *bind.CallOpts) (string, error) {
	var out []interface{}
	err := _MessagePasser.contract.Call(opts, &out, "version")

	if err != nil {
		return *new(string), err
	}

	out0 := *abi.ConvertType(out[0], new(string)).(*string)

	return out0, err

}

// Version is a free data retrieval call binding the contract method 0x54fd4d50.
//
// Solidity: function version() view returns(string)
func (_MessagePasser *MessagePasserSession) Version() (string, error) {
	return _MessagePasser.Contract.Version(&_MessagePasser.CallOpts)
}

// Version is a free data retrieval call binding the contract method 0x54fd4d50.
//
// Solidity: function version() view returns(string)
func (_MessagePasser *MessagePasserCallerSession) Version() (string, error) {
	return _MessagePasser.Contract.Version(&_MessagePasser.CallOpts)
}

// InitiateWithdrawal is a paid mutator transaction binding the contract method 0x6b08a025.
//
// Solidity: function initiateWithdrawal((bytes32,(bytes32,bool,bool)[],bytes)[] ixs) payable returns()
func (_MessagePasser *MessagePasserTransactor) InitiateWithdrawal(opts *bind.TransactOpts, ixs []MessagePasserInstruction) (*types.Transaction, error) {
	return _MessagePasser.contract.Transact(opts, "initiateWithdrawal", ixs)
}

// InitiateWithdrawal is a paid mutator transaction binding the contract method 0x6b08a025.
//
// Solidity: function initiateWithdrawal((bytes32,(bytes32,bool,bool)[],bytes)[] ixs) payable returns()
func (_MessagePasser *MessagePasserSession) InitiateWithdrawal(ixs []MessagePasserInstruction) (*types.Transaction, error) {
	return _MessagePasser.Contract.InitiateWithdrawal(&_MessagePasser.TransactOpts, ixs)
}

// InitiateWithdrawal is a paid mutator transaction binding the contract method 0x6b08a025.
//
// Solidity: function initiateWithdrawal((bytes32,(bytes32,bool,bool)[],bytes)[] ixs) payable returns()
func (_MessagePasser *MessagePasserTransactorSession) InitiateWithdrawal(ixs []MessagePasserInstruction) (*types.Transaction, error) {
	return _MessagePasser.Contract.InitiateWithdrawal(&_MessagePasser.TransactOpts, ixs)
}

// MessagePasserMessagePassedIterator is returned from FilterMessagePassed and is used to iterate over the raw logs and unpacked data for MessagePassed events raised by the MessagePasser contract.
type MessagePasserMessagePassedIterator struct {
	Event *MessagePasserMessagePassed // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *MessagePasserMessagePassedIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(MessagePasserMessagePassed)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(MessagePasserMessagePassed)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *MessagePasserMessagePassedIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *MessagePasserMessagePassedIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// MessagePasserMessagePassed represents a MessagePassed event raised by the MessagePasser contract.
type MessagePasserMessagePassed struct {
	Nonce          *big.Int
	Sender         common.Address
	Ixs            []MessagePasserInstruction
	WithdrawalHash [32]byte
	Raw            types.Log // Blockchain specific contextual infos
}

// FilterMessagePassed is a free log retrieval operation binding the contract event 0x157b3e0c97c86afb7b397c7ce91299a7812096cb61a249153094208e1f74a1b8.
//
// Solidity: event MessagePassed(uint256 indexed nonce, address indexed sender, (bytes32,(bytes32,bool,bool)[],bytes)[] ixs, bytes32 withdrawalHash)
func (_MessagePasser *MessagePasserFilterer) FilterMessagePassed(opts *bind.FilterOpts, nonce []*big.Int, sender []common.Address) (*MessagePasserMessagePassedIterator, error) {

	var nonceRule []interface{}
	for _, nonceItem := range nonce {
		nonceRule = append(nonceRule, nonceItem)
	}
	var senderRule []interface{}
	for _, senderItem := range sender {
		senderRule = append(senderRule, senderItem)
	}

	logs, sub, err := _MessagePasser.contract.FilterLogs(opts, "MessagePassed", nonceRule, senderRule)
	if err != nil {
		return nil, err
	}
	return &MessagePasserMessagePassedIterator{contract: _MessagePasser.contract, event: "MessagePassed", logs: logs, sub: sub}, nil
}

// WatchMessagePassed is a free log subscription operation binding the contract event 0x157b3e0c97c86afb7b397c7ce91299a7812096cb61a249153094208e1f74a1b8.
//
// Solidity: event MessagePassed(uint256 indexed nonce, address indexed sender, (bytes32,(bytes32,bool,bool)[],bytes)[] ixs, bytes32 withdrawalHash)
func (_MessagePasser *MessagePasserFilterer) WatchMessagePassed(opts *bind.WatchOpts, sink chan<- *MessagePasserMessagePassed, nonce []*big.Int, sender []common.Address) (event.Subscription, error) {

	var nonceRule []interface{}
	for _, nonceItem := range nonce {
		nonceRule = append(nonceRule, nonceItem)
	}
	var senderRule []interface{}
	for _, senderItem := range sender {
		senderRule = append(senderRule, senderItem)
	}

	logs, sub, err := _MessagePasser.contract.WatchLogs(opts, "MessagePassed", nonceRule, senderRule)
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(MessagePasserMessagePassed)
				if err := _MessagePasser.contract.UnpackLog(event, "MessagePassed", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseMessagePassed is a log parse operation binding the contract event 0x157b3e0c97c86afb7b397c7ce91299a7812096cb61a249153094208e1f74a1b8.
//
// Solidity: event MessagePassed(uint256 indexed nonce, address indexed sender, (bytes32,(bytes32,bool,bool)[],bytes)[] ixs, bytes32 withdrawalHash)
func (_MessagePasser *MessagePasserFilterer) ParseMessagePassed(log types.Log) (*MessagePasserMessagePassed, error) {
	event := new(MessagePasserMessagePassed)
	if err := _MessagePasser.contract.UnpackLog(event, "MessagePassed", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}
